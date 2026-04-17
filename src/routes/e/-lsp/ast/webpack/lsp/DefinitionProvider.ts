import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { error } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import type { TModuleId } from "@/utils/types";
import { isWebpackModule } from "@vencord-companion/webpack-ast-parser/util";

import { toMonacoRange, toParserPosition } from "../../../util";

export class WebpackDefinitionProvider implements Monaco.languages.DefinitionProvider {
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
            const defs = await parser.generateDefinitions(toParserPosition(position));

            if (!defs) {
                return;
            }

            const monacoDefs: Monaco.languages.Location[] = [];

            for (const def of defs) {
                monacoDefs.push({
                    range: toMonacoRange(def.range),
                    uri: (await getModuleModel(+def.moduleId as TModuleId)).uri,
                });
            }

            return monacoDefs;
        } catch (e) {
            console.error(e);
        }
    }

    public static register() {
        monaco.languages.registerDefinitionProvider({ language: "javascript" }, new WebpackDefinitionProvider());
    }
}
