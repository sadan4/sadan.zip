import type { TBundleHash } from "@/utils/types";

import type { IBuildService, RawBuildService } from "./sharedWorker";

import * as comlink from "comlink";

export type RemoteBuildService = comlink.Remote<IBuildService>;

const workerMap = /* @__PURE__ */ new Map<TBundleHash, RemoteBuildService>();

export async function getBuildService(hash: TBundleHash): Promise<RemoteBuildService> {
    if (workerMap.has(hash)) {
        return workerMap.get(hash)!;
    }

    const worker = new SharedWorker(new URL("./sharedWorker", import.meta.url), {
        name: `build-worker-${hash}`,
        type: "module",
    });

    const buildService = comlink.wrap<RawBuildService>(worker.port);

    await buildService.init(hash);

    workerMap.set(hash, buildService);

    return buildService;
}
