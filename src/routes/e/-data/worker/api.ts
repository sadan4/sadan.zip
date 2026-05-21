import type { TBundleHash } from "@/utils/types";

import type { IBuildService, RawBuildService } from "./sharedWorker";

import * as comlink from "comlink";
import prodWorkerUrl from "omt:./sharedWorker";

let workerUrl: string;

// OMT doesn't work in dev
if (!import.meta.env.SSR && import.meta.env.DEV) {
    ({ default: workerUrl } = await import("./sharedWorker?sharedworker&url"));
} else {
    workerUrl = prodWorkerUrl;
}

// union with actual BuildService to jump to the implementation instead of the interface def
export type RemoteBuildService = comlink.Remote<IBuildService | RawBuildService>;

const workerMap = /* @__PURE__ */ new Map<TBundleHash, RemoteBuildService>();

export async function getBuildService(hash: TBundleHash): Promise<RemoteBuildService> {
    if (workerMap.has(hash)) {
        return workerMap.get(hash)!;
    }

    const worker = new SharedWorker(workerUrl, {
        name: `build-worker-${hash}`,
        type: "module",
        extendedLifetime: !import.meta.env.DEV, // in dev we want fast reloads
    });

    const buildService = comlink.wrap<RawBuildService>(worker.port);

    await buildService.init(hash);

    workerMap.set(hash, buildService);

    return buildService;
}
