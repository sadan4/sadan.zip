import { getRouter } from "@/router";
import { unreachable } from "@/utils/error";
import { once } from "@/utils/functional";
import { type Monaco, monaco } from "@/utils/monaco";
import { entries, mapValues } from "@/utils/obj";
import { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";

import { WebpackExportHover } from "./ast/webpack/hover/ExportHover";
import { WebpackI18nHover } from "./ast/webpack/hover/I18nHover";
import { WebpackDefinitionProvider } from "./ast/webpack/lsp/DefinitionProvider";
import type { DepsJson, TModuleId } from "../../../../server/types";
import { getModuleURI, ModuleViewerSettingsStore, ModuleViewerStore, parseModuleURI } from "../-data";

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
            // FIXME: ensure this model is available when monaco loads it
            return getModuleURI(buildHash, id as TModuleId).toString();
        },
        getModuleParser(_requestor, id, _latest) {
            return ModuleViewerStore.getState().getModuleParser(id as TModuleId);
        },
    });

    WebpackExportHover.register();
    WebpackI18nHover.register();
    WebpackDefinitionProvider.register();

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

            const search = selectionOrPosition
                ? monaco.Range.isIRange(selectionOrPosition)
                    ? {
                        sl: selectionOrPosition.startLineNumber,
                        sc: selectionOrPosition.startColumn,
                        el: selectionOrPosition.endLineNumber,
                        ec: selectionOrPosition.endColumn,
                    } as const
                    : {
                        sl: selectionOrPosition.lineNumber,
                        sc: selectionOrPosition?.column,
                        el: selectionOrPosition.lineNumber,
                        ec: selectionOrPosition?.column,
                    } as const
                : undefined;

            if (ModuleViewerSettingsStore.getState().openModulesInNewTab) {
                // FIXME: look into Router.buildLocation
                const url = new URL(
                    `/e/view/${encodeURIComponent(parsed.buildHash)}/${encodeURIComponent(parsed.moduleId)}`,
                    location.origin,
                );

                if (search) {
                    for (const [key, value] of entries(mapValues(search, String))) {
                        url.searchParams.set(key, value);
                    }
                }

                window.open(url.toString(), "_blank", "noopener,noreferrer");
            } else {
                getRouter().navigate({
                    to: "/e/view/{-$buildHash}/{-$moduleId}",
                    params: {
                        buildHash: parsed.buildHash,
                        moduleId: parsed.moduleId,
                    },
                    search,
                });
            }

            return true;
        }
    }());
}

export const registerLSPHandlers = once(_register);
