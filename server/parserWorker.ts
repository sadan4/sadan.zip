import { chunk } from "@/utils/array";
import { assert, error } from "@/utils/error";
import { GlobalEnvParser } from "@vencord-companion/global-env-parser";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser";
import { WebpackLazyChunkParser, WebpackMainChunkParser } from "@vencord-companion/webpack-chunk-parser";

import { BUILDS_PATH, type Channels, SYM_CJS_DEFAULT_PLACEHOLDER } from "./constants";
import type { BundleInfo, DepsJson, KeyModules, MainDeps, ModuleInfo, TBundleHash, TModuleId } from "./types";
import { entries, fetchAsset, keys } from "./utils";

import { exists } from "fs-extra";
import { JSDOM } from "jsdom";
import { link, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { workerData } from "node:worker_threads";

export interface ParserWorkerData {
    buildHash: TBundleHash;
    html: string;
    channel: Channels;
}

async function getChunkText(channel: Channels, hash: string) {
    const path = join(BUILDS_PATH, "chunks", `${hash}`, "raw.js");

    if (await exists(path)) {
        return readFile(path, "utf8");
    }

    await mkdir(dirname(path), { recursive: true });

    const res = await fetchAsset(channel, `${hash}.js`);
    const text = await res.text();

    writeFile(path, text);

    return text;
}

/**
 * map of module id -> module source
 */
const moduleMap = new Map<TModuleId, string>();
// time markers
const PARSING_MAIN_JS_TIME = "Parsing web.js";
const PARSING_LAZY_CHUNKS_TIME = "Parsing lazy chunks";

async function findBuildModules({ buildHash, html, channel }: ParserWorkerData) {
    const { window: { document: doc } } = new JSDOM(html);
    const scriptElements = [...doc.querySelectorAll("script")];
    const envScriptEl = scriptElements.find((el) => el.textContent?.includes("window.GLOBAL_ENV =")) ?? error("Could not find env script element");
    const parser = new GlobalEnvParser(envScriptEl.textContent!);

    // parse env vars to ensure they're valid
    parser.getGlobalEnvObject() ?? error("Could not parse global env vars");

    const entryScriptNames = scriptElements
        .map(({ src }) => src)
        .filter((pathName) => pathName.startsWith("/assets/"))
        .map((pathName) => pathName.replace("/assets/", ""));

    const webJsUrl = entryScriptNames.find((name) => name.startsWith("web.")) ?? error("Could not find web.js entrypoint");
    const webJsContent = await (await fetchAsset(channel, webJsUrl)).text();

    console.time(PARSING_MAIN_JS_TIME);

    const mainParser = new WebpackMainChunkParser(webJsContent);
    const buildNumber = mainParser.getBuildNumber() ?? error("Could not find build number");
    const initialModules: Record<TModuleId, string> = mainParser.getDefinedModules() ?? error("could not parse main chunk");

    console.timeEnd(PARSING_MAIN_JS_TIME);

    const modules: ModuleInfo = {
        [webJsUrl]: keys(initialModules),
    };

    const writes = [] as Promise<void>[];
    const MODULES_PATH = join(BUILDS_PATH, buildHash, ".modules");

    await mkdir(MODULES_PATH);
    for (const [moduleId, moduleSource] of entries(initialModules)) {
        moduleMap.set(moduleId, moduleSource);
        writes.push(writeFile(join(MODULES_PATH, `${moduleId}.js`), moduleSource, "utf8"));
    }

    const chunkHashes = mainParser.getJsChunkHashes();

    console.time(PARSING_LAZY_CHUNKS_TIME);

    // chunk the array to not send 1500 requests at once
    for (const batch of chunk(chunkHashes, 50)) {
        await Promise.all(batch.map(async ([chunkId, hash]) => {
            try {
                const text = await getChunkText(channel, hash);

                if (text.includes(`.ruid="`)) {
                    // worker chunk, not part of main bundle
                    return;
                }

                const parser = new WebpackLazyChunkParser(text);
                const selfModules: Record<TModuleId, string> | undefined = parser.getDefinedModules();

                if (!selfModules) {
                    error("could not parse lazy chunk");
                }

                modules[`${hash}.js`] = keys(selfModules);

                for (const [moduleId, moduleSource] of entries(selfModules)) {
                    const path = join(BUILDS_PATH, "chunks", hash, `${moduleId}.js`);
                    const targetPath = resolve(join(MODULES_PATH, `${moduleId}.js`));

                    writes.push(writeFile(path, moduleSource, "utf8")
                        .then(() => link(resolve(path), targetPath)
                            .catch((e) => {
                                if ((e as NodeJS.ErrnoException).code !== "EEXIST") {
                                    throw e;
                                }
                            })));
                    moduleMap.set(moduleId, moduleSource);
                }
            } catch (e) {
                console.error(`Error fetching/parsing chunk ${chunkId} with hash ${hash} on channel ${channel}`);
                console.error(e);
            }
        }));
    }

    console.timeEnd(PARSING_LAZY_CHUNKS_TIME);


    const bundleInfo: BundleInfo = {
        buildHash,
        buildNumber,
        modules,
        envVarText: parser.text,
        firstSeen: Date.now(),
    };

    writes.push(writeFile(join(BUILDS_PATH, buildHash, "info.json"), JSON.stringify(bundleInfo, null, 2)));

    const FS_WRITE_TIME = `Writing ${writes.length} files to disk`;

    console.time(FS_WRITE_TIME);

    await Promise.all(writes);

    console.timeEnd(FS_WRITE_TIME);
}

const MAKE_DEP_GRAPH_TIME = "Making dependency graph";

function makeDependencyGraph(): [MainDeps, WebpackAstParser[]] {
    console.time(MAKE_DEP_GRAPH_TIME);

    const deps: MainDeps = {};
    const parsers = [] as WebpackAstParser[];

    for (const [moduleId, text] of moduleMap) {
        try {
            const parser = WebpackAstParser.withModule(text, moduleId);

            parsers.push(parser);

            const { sync = [], lazy = [] } = parser.getModulesThatThisModuleRequires() ?? {};

            for (const depModuleId of sync) {
                (deps[depModuleId as TModuleId] ??= {
                    lazyUses: [],
                    syncUses: [],
                }).syncUses.push(moduleId);
            }
            for (const depModuleId of lazy) {
                (deps[depModuleId as TModuleId] ??= {
                    lazyUses: [],
                    syncUses: [],
                }).lazyUses.push(moduleId);
            }
        } catch (e) {
            console.error("Error parsing module for dependency graph:", moduleId);
            throw e;
        }
    }

    console.timeEnd(MAKE_DEP_GRAPH_TIME);
    return [deps, parsers];
}

const KEY_MODULES_TIME = "Finding Key Modules";

async function findKeyModules(parsers: WebpackAstParser[]): Promise<KeyModules> {
    const ret: KeyModules = {
        fluxDispatcherClass: [],
    };

    console.time(KEY_MODULES_TIME);

    for (const parser of parsers) {
        try {
            // Flux Dispatcher
            {
                const fluxDispatcherModuleExport = parser.isFluxDispatcherModule();

                if (fluxDispatcherModuleExport != null) {
                    if (parser.moduleId == null) {
                        throw new Error("Module ID is not set for module");
                    }
                    ret.fluxDispatcherClass.push([parser.moduleId as TModuleId, fluxDispatcherModuleExport]);

                    const arr = (await parser.getAllReExportsForExport(fluxDispatcherModuleExport))
                        .filter(([, exportChain]) => exportChain.length === 1);

                    for (let [moduleId, [exportName]] of arr) {
                        if (typeof exportName === "symbol") {
                            assert(exportName === WebpackAstParser.SYM_CJS_DEFAULT);
                            exportName = SYM_CJS_DEFAULT_PLACEHOLDER;
                        }
                        ret.fluxDispatcherClass.push([moduleId as TModuleId, exportName]);
                    }
                }
            }
        } catch (e) {
            console.error("Error finding key modules:", parser.moduleId);
            console.error(e);
            throw e;
        }
    }
    console.timeEnd(KEY_MODULES_TIME);
    return ret;
}

async function processBuild(data: ParserWorkerData) {
    await findBuildModules(data);

    const [deps, parsers] = makeDependencyGraph();

    WebpackAstParser.setDefaultModuleCache({
        getLatestModuleFromNum(id) {
            return Promise.resolve(moduleMap.get(`${id}` as TModuleId) ?? error(`module not found: ${id}`));
        },
        getModuleFromNum(id) {
            return Promise.resolve(moduleMap.get(id as TModuleId) ?? error(`module not found: ${id}`));
        },
        getModuleFilepath(_id) {
            return undefined;
        },
    });
    WebpackAstParser.setDefaultModuleDepManager({
        getModDeps(moduleId) {
            return deps[moduleId as TModuleId] ?? error(`module not found: ${moduleId}`);
        },
    });


    const keyModules = await findKeyModules(parsers);

    await writeFile(
        join(BUILDS_PATH, data.buildHash, "deps.json"),
        JSON.stringify({
            deps,
            keyModules,
        } satisfies DepsJson, null, 2),
        "utf8",
    );

    // we should exit right after this, but might as well free the memory
    moduleMap.clear();
}

processBuild(workerData as ParserWorkerData).catch((e) => {
    console.error("Failed to process build in parser worker:");
    console.error(e);
});
