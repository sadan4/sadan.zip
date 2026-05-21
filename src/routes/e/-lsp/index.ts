import { getRouter } from "@/router";
import { once } from "@/utils/functional";
import { type Monaco, monaco } from "@/utils/monaco";
import { entries, mapValues } from "@/utils/obj";

import { WebpackExportHover } from "./ast/webpack/hover/ExportHover";
import { WebpackDefinitionProvider } from "./ast/webpack/lsp/DefinitionProvider";
import { WebpackReferenceProvider } from "./ast/webpack/lsp/ReferenceProvider";
import { ModuleViewerSettingsStore, parseModuleURI } from "../-data";

function _register() {
    WebpackExportHover.register();
    WebpackDefinitionProvider.register();
    WebpackReferenceProvider.register();

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
                        moduleId: +parsed.moduleId,
                    },
                    search,
                });
            }

            return true;
        }
    }());
}

export const registerLSPHandlers = once(_register);
