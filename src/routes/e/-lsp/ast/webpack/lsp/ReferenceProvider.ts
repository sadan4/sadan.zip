import type { RemoteBuildService } from "@/routes/e/-data/worker/api";
import type { ModuleLocation } from "@/routes/e/-data/worker/sharedWorker";
import { type Monaco, monaco } from "@/utils/monaco";
import type { TModuleId } from "@/utils/types";

import { ProviderBase } from "./ProviderBase";

export class WebpackReferenceProvider extends ProviderBase implements Monaco.languages.ReferenceProvider {
    private constructor() {
        super();
    }

    protected override generateLocations(
        buildService: RemoteBuildService,
        moduleId: TModuleId,
        position: Monaco.Position,
    ): Promise<ModuleLocation[] | null> {
        return buildService.generateReferences(moduleId, position);
    }

    async provideReferences(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _context: Monaco.languages.ReferenceContext,
        _token: Monaco.CancellationToken,
    ): Promise<Monaco.languages.Location[] | null | undefined> {
        try {
            return this.provide(model, position);
        } catch (e) {
            console.error(e);
        }
        return Promise.resolve(undefined);
    }

    public static register() {
        monaco.languages.registerReferenceProvider({ language: "javascript" }, new WebpackReferenceProvider());
    }
}
