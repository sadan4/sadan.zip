import { unreachable } from "@/utils/error";
import { once } from "@/utils/functional";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";

import { DefinitionProvider } from "./ast/webpack/lsp/DefinitionProvider";
import type { DepsJson, TModuleId } from "../../../../server/types";
import { getModuleURI, ModuleViewerStore } from "../-data";

async function _register() {
    let { buildHash, getDepsGraph } = ModuleViewerStore.getState();
    let depGraph: DepsJson | null = buildHash ? await getDepsGraph() : null;

    ModuleViewerStore.subscribe(async (state, _prevState) => {
        if (state.buildHash !== buildHash) {
            buildHash = state.buildHash;
            depGraph = await ModuleViewerStore.getState().getDepsGraph();
        }
    });

    WebpackAstParser.setDefaultModuleDepManager({
        getModDeps(moduleId) {
            if (!depGraph) {
                unreachable();
            }
            return depGraph.deps[moduleId as TModuleId];
        },
    });

    WebpackAstParser.setDefaultModuleCache({
        getModuleFilepath(id) {
            return getModuleURI(buildHash, id as TModuleId).toString();
        },
        getModuleParser(_requestor, id, _latest) {
            return ModuleViewerStore.getState().getModuleParser(id as TModuleId);
        },
    });

    DefinitionProvider.register();
}

export const registerLSPHandlers = once(_register);
