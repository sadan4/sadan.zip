import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { error } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import { isWebpackModule } from "@vencord-companion/webpack-ast-parser/util";

import { toMonacoRange, toParserPosition } from "../../../util";

export class WebpackExportHover implements Monaco.languages.HoverProvider {
    private constructor() {
    }

    async provideHover(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _token: Monaco.CancellationToken,
        _context?: Monaco.languages.HoverContext<Monaco.languages.Hover> | undefined,
    ): Promise<Monaco.languages.Hover | null | undefined> {
        try {
            const { buildHash, getModuleParser } = ModuleViewerStore.getState();
            const parsedUri = parseModuleURI(model.uri);
            const text = model.getValue();

            if (parsedUri?.buildHash !== buildHash) {
                error("Build hash mismatch");
            }
            if (!parsedUri || !isWebpackModule(text)) {
                return;
            }

            const parser = await getModuleParser(parsedUri.moduleId);
            const [range, hoverText] = await parser.generateHover(toParserPosition(position)) ?? [];

            // also catches empty string for hoverText
            if (!hoverText) {
                return;
            }
            return {
                range: toMonacoRange(range!),
                contents: [
                    {
                        value: hoverText,
                    },
                ],
            };
        } catch (e) {
            console.error("[WebpackExportHover]:", e);
        }
    }

    public static register() {
        monaco.languages.registerHoverProvider({ language: "javascript" }, new WebpackExportHover());
    }
}
