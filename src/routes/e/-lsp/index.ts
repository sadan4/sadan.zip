import { getRouter } from "@/router";
import { unreachable } from "@/utils/error";
import { once } from "@/utils/functional";
import { type Monaco, monaco } from "@/utils/monaco";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";

import { DefinitionProvider } from "./ast/webpack/lsp/DefinitionProvider";
import type { DepsJson, TModuleId } from "../../../../server/types";
import { getModuleURI, ModuleViewerStore, parseModuleURI } from "../-data";

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

    monaco.editor.registerEditorOpener(new class implements Monaco.editor.ICodeEditorOpener {
        openCodeEditor(
            _source: Monaco.editor.ICodeEditor,
            resource: Monaco.Uri,
            selectionOrPosition?: Monaco.IRange | Monaco.IPosition,
        ): boolean | Promise<boolean> {
            const parsed = parseModuleURI(resource);

            if (!parsed) {
                return false;
            }

            console.log("Opening module", parsed.moduleId, "in build", parsed.buildHash);

            getRouter().navigate({
                to: "/e/view/{-$buildHash}/{-$moduleId}",
                params: {
                    buildHash: parsed.buildHash,
                    moduleId: parsed.moduleId,
                },
                search: selectionOrPosition
                    ? monaco.Range.isIRange(selectionOrPosition)
                        ? {
                            sl: selectionOrPosition.startLineNumber,
                            sc: selectionOrPosition.startColumn,
                            el: selectionOrPosition.endLineNumber,
                            ec: selectionOrPosition.endColumn,
                        }
                        : {
                            sl: selectionOrPosition.lineNumber,
                            sc: selectionOrPosition?.column,
                            el: selectionOrPosition.lineNumber,
                            ec: selectionOrPosition?.column,
                        }
                    : undefined,
            });

            return false;
        }
    }());
}

export const registerLSPHandlers = once(_register);
