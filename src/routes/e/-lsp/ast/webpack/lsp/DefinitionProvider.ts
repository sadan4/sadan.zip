import type { RemoteBuildService } from "@/routes/e/-data/worker/api";
import type { ModuleLocation } from "@/routes/e/-data/worker/sharedWorker";
import { type Monaco, monaco } from "@/utils/monaco";
import type { TModuleId } from "@/utils/types";

import { ProviderBase } from "./ProviderBase";

export class WebpackDefinitionProvider extends ProviderBase implements Monaco.languages.DefinitionProvider {
    private constructor() {
        super();
    }

    protected generateLocations(
        buildService: RemoteBuildService,
        moduleId: TModuleId,
        position: Monaco.Position,
    ): Promise<ModuleLocation[] | null> {
        return buildService.generateDefinitions(moduleId, position);
    }

    provideDefinition(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _token: Monaco.CancellationToken,
    ): Promise<Monaco.languages.Definition | null | undefined> {
        try {
            return this.provide(model, position);
        } catch (e) {
            console.error(e);
        }
        return Promise.resolve<undefined>(undefined);
    }

    public static register() {
        monaco.languages.registerDefinitionProvider({ language: "javascript" }, new WebpackDefinitionProvider());
    }
}
