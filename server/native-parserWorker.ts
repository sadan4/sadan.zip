import { chunk } from "@/utils/array";
import { assert, error } from "@/utils/error";
import { entries, keys } from "@/utils/obj";
import { GlobalEnvParser } from "@vencord-companion/global-env-parser";
import { TAssert, WebpackAstParser } from "@vencord-companion/webpack-ast-parser";
import { WebpackLazyChunkParser, WebpackMainChunkParser } from "@vencord-companion/webpack-chunk-parser";

import { type Channels } from "./constants";
import native from "./native";
import { MainDeps, type TBundleHash, TModuleId } from "./types";
import { fetchAsset } from "./utils";

import { JSDOM } from "jsdom";
import { workerData } from "node:worker_threads";

export interface ParserWorkerData {
    buildHash: TBundleHash;
    html: string;
    channel: Channels;
}

async function getChunkText(channel: Channels, hash: string) {
    const res = await fetchAsset(channel, `${hash}.js`);

    return await res.text();
}

// time markers
const PARSING_MAIN_JS_TIME = "Parsing web.js";
const PARSING_LAZY_CHUNKS_TIME = "Parsing lazy chunks";

async function findBuildModules(
    { buildHash, html, channel }: ParserWorkerData,
    moduleMap: Map<TModuleId, string>,
    pBuild: native.ProcessingBuild,
) {
    const { window: { document: doc } } = new JSDOM(html);
    const scriptElements = [...doc.querySelectorAll("script")];
    const envScriptEl = scriptElements.find((el) => el.textContent?.includes("window.GLOBAL_ENV =")) ?? error("Could not find env script element");
    const buildMetadata = new native.ProcessingMetadata();
    const parser = new GlobalEnvParser(envScriptEl.textContent!);

    buildMetadata.setFirstSeen(Date.now());
    buildMetadata.envVarText = parser.text;
    buildMetadata.buildHash = buildHash;

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
    const entryPoint = (mainParser.getEntrypointId() ?? error("Could not find entry point module id")) as TModuleId;
    const buildNumber = mainParser.getBuildNumber() ?? error("Could not find build number");
    const initialModules: Record<TModuleId, string> = mainParser.getDefinedModules() ?? error("could not parse main chunk");

    buildMetadata.entryPoint = +entryPoint;
    buildMetadata.buildNumber = +buildNumber;

    console.timeEnd(PARSING_MAIN_JS_TIME);

    pBuild.setModuleSources(webJsUrl, keys(initialModules).map((id) => +id));

    for (const [moduleId, moduleSource] of entries(initialModules)) {
        moduleMap.set(moduleId, moduleSource);
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

                pBuild.setModuleSources(`${hash}.js`, keys(selfModules).map((id) => +id));

                for (const [moduleId, moduleSource] of entries(selfModules)) {
                    moduleMap.set(moduleId, moduleSource);
                }
            } catch (e) {
                console.error(`Error fetching/parsing chunk ${chunkId} with hash ${hash} on channel ${channel}`);
                console.error(e);
            }
        }));
    }

    console.timeEnd(PARSING_LAZY_CHUNKS_TIME);

    pBuild.metadata = buildMetadata;
}

const MAKE_DEP_GRAPH_TIME = "Making dependency graph";

function makeDependencyGraph(
    moduleMap: Map<TModuleId, string>,
    parserCache: Map<TModuleId, WebpackAstParser>,
    pDepInfo: native.ProcessingDepInfo,
): [deps: MainDeps, parsers: WebpackAstParser[]] {
    console.time(MAKE_DEP_GRAPH_TIME);

    /**
    * map of module id -> module source
    */
    const deps: MainDeps = {};
    const parsers = [] as WebpackAstParser[];

    for (const [moduleId, text] of moduleMap) {
        try {
            const parser = parserCache.get(moduleId) ?? WebpackAstParser.withModule(text, moduleId);

            parserCache.set(moduleId, parser);
            parsers.push(parser);

            const { sync = [], lazy = [] } = parser.getModulesThatThisModuleRequires() ?? {};

            for (const depModuleId of sync) {
                pDepInfo.addSyncDep(+depModuleId, +moduleId);
                (deps[depModuleId as TModuleId] ??= {
                    lazyUses: [],
                    syncUses: [],
                }).syncUses.push(moduleId);
            }
            for (const depModuleId of lazy) {
                pDepInfo.addLazyDep(+depModuleId, +moduleId);
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

async function findKeyModules(parsers: WebpackAstParser[]): Promise<native.ProcessingKeyModules> {
    const pKeyModules = new native.ProcessingKeyModules();

    console.time(KEY_MODULES_TIME);

    for (let i = 0; i < parsers.length; ++i) {
        const parser = parsers[i];

        try {
            // Flux Dispatcher
            {
                const fluxDispatcherModuleExport = parser.isFluxDispatcherModule();

                if (fluxDispatcherModuleExport != null) {
                    if (parser.moduleId == null) {
                        throw new Error("Module ID is not set for module");
                    }
                    pKeyModules.addFluxDispatcherClass(+parser.moduleId, {
                        type: "Named",
                        field0: fluxDispatcherModuleExport,
                    });

                    const arr = (await parser.getAllReExportsForExport(fluxDispatcherModuleExport))
                        .filter(([, exportChain]) => exportChain.length === 1);

                    for (const [moduleId, [exportName]] of arr) {
                        if (typeof exportName === "symbol") {
                            assert(exportName === WebpackAstParser.SYM_CJS_DEFAULT);
                            pKeyModules.addFluxDispatcherClass(+moduleId, { type: "Default" });
                        } else {
                            pKeyModules.addFluxDispatcherClass(+moduleId, {
                                type: "Named",
                                field0: exportName,
                            });
                        }
                    }
                }
            }
        } catch (e) {
            console.error("Error finding key modules:", parser.moduleId);
            console.error(e);
            throw e;
        }
        delete parsers[i]; // free memory as we go
    }
    console.timeEnd(KEY_MODULES_TIME);
    return pKeyModules;
}

async function processBuild(data: ParserWorkerData) {
    const pBuild = new native.ProcessingBuild();
    const moduleMap: Map<TModuleId, string> = new Map();

    await findBuildModules(data, moduleMap, pBuild);

    const parserCache = new Map<TModuleId, WebpackAstParser>();
    const pDepInfo = new native.ProcessingDepInfo();
    let [deps, parsers] = makeDependencyGraph(moduleMap, parserCache, pDepInfo);

    WebpackAstParser.setDefaultModuleCache({
        getModuleFilepath(_id) {
            return undefined;
        },
        getModuleParser(_requestor, id, _latest) {
            TAssert<TModuleId>(id);
            if (!parserCache.has(id)) {
                parserCache.set(id, WebpackAstParser.withFormattedModule(moduleMap.get(id) ?? error(`module not found: ${id}`), id));
            }
            return Promise.resolve(parserCache.get(id)!);
        },
    });
    WebpackAstParser.setDefaultModuleDepManager({
        getModDeps(moduleId) {
            return deps[moduleId as TModuleId] ?? error(`module not found: ${moduleId}`);
        },
    });


    pDepInfo.keyModules = await findKeyModules(parsers);
    parsers.length = 0;
    deps = {};
    for (const [id, source] of moduleMap) {
        pBuild.setSource(+id, source);
        moduleMap.delete(id); // free as we go
    }

    pBuild.depInfo = pDepInfo;
    pBuild.write();

    console.log(`Finished processing build ${data.buildHash}`);
}

processBuild(workerData as ParserWorkerData).catch((e) => {
    console.error("Failed to process build in parser worker:");
    console.error(e);
});
