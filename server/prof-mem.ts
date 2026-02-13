import { assert, error } from "@/utils/error";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";

import { SYM_CJS_DEFAULT_PLACEHOLDER } from "./constants";
import type { DepsJson, KeyModules, MainDeps, TModuleId } from "./types";

import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const BUILD_PATH = "C:/Users/meyer/dev/html/sadan.zip/builds/8d3317a97ab736931dc8f8a1cb4bd67d8384766a";
const MODULES_PATH = join(BUILD_PATH, ".modules");
const MAKE_DEP_GRAPH_TIME = "Making dependency graph";
const KEY_MODULES_TIME = "Finding Key Modules";
const COLLECT_MODULES_TIME = "Collecting modules";
const ALL_TIME = "Memory benchmark";

async function collectModules(): Promise<Map<TModuleId, string>> {
    console.time(COLLECT_MODULES_TIME);

    const ret = new Map<TModuleId, string>();

    for (const file of await readdir(MODULES_PATH)) {
        const moduleId = file.slice(0, -3) as TModuleId;
        const text = await readFile(join(MODULES_PATH, file), "utf8");

        ret.set(moduleId, text);
    }

    console.timeEnd(COLLECT_MODULES_TIME);
    return ret;
}

function makeDependencyGraph(moduleMap: Map<TModuleId, string>): [MainDeps, WebpackAstParser[]] {
    console.time(MAKE_DEP_GRAPH_TIME);

    /**
    * map of module id -> module source
    */
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

async function findKeyModules(parsers: WebpackAstParser[]): Promise<KeyModules> {
    const ret: KeyModules = {
        fluxDispatcherClass: [],
    };

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
        delete parsers[i]; // free memory as we go
    }
    console.timeEnd(KEY_MODULES_TIME);
    return ret;
}

!async function () {
    console.log("running memory benchmark...");
    console.time(ALL_TIME);

    const moduleMap = await collectModules();
    //
    const [depGraph, parsers] = makeDependencyGraph(moduleMap);

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
            return depGraph[moduleId as TModuleId] ?? error(`module not found: ${moduleId}`);
        },
    });

    const keyModules = await findKeyModules(parsers);

    if (!(Math.random() || Math.random())) {
        await writeFile("how.json", JSON.stringify({
            deps: depGraph,
            keyModules,
        } satisfies DepsJson, null, 2));
    }

    console.timeEnd(ALL_TIME);
}();
