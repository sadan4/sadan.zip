import { getModuleURI, ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { error, unreachable } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import { Position as WP_Position } from "@vencord-companion/shared/Position";
import { isWebpackModule } from "@vencord-companion/webpack-ast-parser/util";

import type { TModuleId } from "../../../../../../../server/types";
import { toMoancoRange } from "../../../util";

export class DefinitionProvider implements Monaco.languages.DefinitionProvider {
    private constructor() {
    }

    async provideDefinition(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _token: Monaco.CancellationToken,
    ): Promise<Monaco.languages.Definition | null | undefined> {
        try {
            const text = model.getValue();

            if (!isWebpackModule(text)) {
                return;
            }

            const { buildHash: currentBuildHash, getModuleParser, getModuleModel } = ModuleViewerStore.getState();
            const { buildHash, moduleId } = parseModuleURI(model.uri) ?? {};

            if (buildHash !== currentBuildHash) {
                error("Build hash mismatch");
            }
            if (!moduleId) {
                return;
            }

            const parser = await getModuleParser(moduleId);
            const pos = new WP_Position(position.lineNumber - 1, position.column - 1);
            const defs = await parser.generateDefinitions(pos);

            if (!defs) {
                return;
            }

            const monacoDefs: Monaco.languages.Location[] = [];

            for (const def of defs) {
                switch (def.locationType) {
                    case "file_path":
                        monacoDefs.push({
                            range: toMoancoRange(def.range),
                            uri: monaco.Uri.file(def.filePath),
                        });
                        break;
                    case "inline":
                        monacoDefs.push({
                            range: toMoancoRange(def.range),
                            uri: (await getModuleModel(def.moduleId as TModuleId)).uri,
                        });
                        break;
                    default:
                        unreachable();
                }
            }

            return monacoDefs;
        } catch (e) {
            console.error(e);
        }
    }

    public static register() {
        monaco.languages.registerDefinitionProvider({ language: "javascript" }, new this());
    }
}
