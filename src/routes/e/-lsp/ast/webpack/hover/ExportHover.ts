import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { error } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import { isWebpackModule } from "@vencord-companion/webpack-ast-parser/util";


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
            const { buildHash: currentBuildHash, _buildService } = ModuleViewerStore.getState();
            const { buildHash, moduleId } = parseModuleURI(model.uri) ?? {};
            const text = model.getValue();

            if (!isWebpackModule(text)) {
                return;
            }

            if (buildHash !== currentBuildHash) {
                error("Build hash mismatch");
            }

            const { range, content } = await _buildService!.generateHover(moduleId!, position) ?? {};

            // also catches empty string for hoverText
            if (!content) {
                return;
            }
            return {
                range: range!,
                contents: [
                    {
                        value: content,
                    },
                ],
            };
        } catch (e) {
            console.error(e);
        }
    }

    public static register() {
        monaco.languages.registerHoverProvider({ language: "javascript" }, new WebpackExportHover());
    }
}
