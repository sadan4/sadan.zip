import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import type { RemoteBuildService } from "@/routes/e/-data/worker/api";
import type { ModuleLocation } from "@/routes/e/-data/worker/sharedWorker";
import { error } from "@/utils/error";
import type { Monaco } from "@/utils/monaco";
import type { TModuleId } from "@/utils/types";

import { isWebpackModule } from "../../../util";

export abstract class ProviderBase {
    protected abstract generateLocations(
        buildService: RemoteBuildService,
        moduleId: TModuleId,
        position: Monaco.Position
    ): Promise<ModuleLocation[] | null>;
    protected async provide(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
    ): Promise<Monaco.languages.Location[] | undefined> {
        const text = model.getValue();

        if (!isWebpackModule(text)) {
            return;
        }

        const { buildHash: currentBuildHash, getModuleModel, _buildService } = ModuleViewerStore.getState();
        const { buildHash, moduleId } = parseModuleURI(model.uri) ?? {};

        if (buildHash !== currentBuildHash) {
            error("Build hash mismatch");
        }

        if (!moduleId) {
            return;
        }

        const defs = await this.generateLocations(_buildService!, moduleId, position);

        if (!defs) {
            return;
        }

        const monacoDefs: Monaco.languages.Location[] = [];

        for (const { id, range } of defs) {
            // await the model to ensure that it is loaded
            const { uri } = await getModuleModel(id as TModuleId);

            monacoDefs.push({
                range,
                uri,
            });
        }

        return monacoDefs;
    }
}
