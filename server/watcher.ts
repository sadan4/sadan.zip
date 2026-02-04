import { appUrl, BUILDS_PATH, Channels } from "./constants";
import type { ParserWorkerData } from "./parserWorker";

import { join } from "path";

import { exists, mkdir } from "fs-extra";
import { Worker } from "node:worker_threads";

const BUILD_ID_HEADER = "x-build-id";

async function checkBuild(url: string): Promise<Response | undefined> {
    const res = await fetch(url);
    const buildId = res.headers.get(BUILD_ID_HEADER);

    if (!buildId) {
        throw new Error("Build ID not in response headers");
    }

    const bundlePath = join(BUILDS_PATH, buildId);

    if (!await exists(bundlePath)) {
        await mkdir(bundlePath);
        return res;
    }
    // we already handled this build
    return;
}

async function checkBuilds() {
    let res: Response | undefined;

    // eslint-disable-next-line no-cond-assign
    if (res = await checkBuild(appUrl[Channels.STABLE])) {
        new Worker(new URL("./parserWorker", import.meta.url), {
            workerData: {
                buildHash: res.headers.get(BUILD_ID_HEADER)!,
                html: await res.text(),
                channel: Channels.STABLE,
            } satisfies ParserWorkerData,
        });
    }

    setTimeout(checkBuilds, 5 * 1000);
}
export function startWatching() {
    mkdir(join(BUILDS_PATH, "chunks"), { recursive: true });
    checkBuilds();
}

startWatching();
