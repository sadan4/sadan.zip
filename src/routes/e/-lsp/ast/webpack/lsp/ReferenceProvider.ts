import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { unreachable } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import { isWebpackModule } from "@vencord-companion/webpack-ast-parser/util";

import type { TModuleId } from "../../../../../../../server/types";
import { toMonacoRange, toParserPosition } from "../../../util";

export class WebpackReferenceProvider implements Monaco.languages.ReferenceProvider {
    private constructor() { }

    async provideReferences(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _context: Monaco.languages.ReferenceContext,
        _token: Monaco.CancellationToken,
    ): Promise<Monaco.languages.Location[] | null | undefined> {
        try {
            const { buildHash, getModuleParser, getModuleModel } = ModuleViewerStore.getState();
            const parsedUri = parseModuleURI(model.uri);
            const text = model.getValue();

            if (parsedUri?.buildHash !== buildHash) {
                return;
            }
            if (!isWebpackModule(text)) {
                return;
            }

            const parser = await getModuleParser(parsedUri.moduleId);
            const refs = await parser.generateReferences(toParserPosition(position));
            const monacoRefs: Monaco.languages.Location[] = [];

            if (!refs) {
                return;
            }

            for (const ref of refs) {
                switch (ref.locationType) {
                    case "file_path": {
                        monacoRefs.push({
                            range: toMonacoRange(ref.range),
                            uri: (await getModuleModel(ref.moduleId as TModuleId)).uri,
                        });
                        break;
                    }
                    case "inline": {
                        monacoRefs.push({
                            range: toMonacoRange(ref.range),
                            uri: (await getModuleModel(ref.moduleId as TModuleId)).uri,
                        });
                        break;
                    }
                    default: {
                        unreachable();
                    }
                }
            }

            return monacoRefs;
        } catch (e) {
            console.error("[WebpackReferenceProvider]", e);
        }
    }

    public static register() {
        monaco.languages.registerReferenceProvider({ language: "javascript" }, new this());
    }
}
