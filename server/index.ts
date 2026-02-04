import { assert, error } from "@/utils/error";

import { BUILDS_PATH } from "./constants";
import type { AllBundleFilesResponseMessage, BundleDepGraphResponseMessage, BundleFileResponseMessage, BundleInfo, BundleMetadataResponseMessage, BundlesResponseMessage, DepsJson, ErrorMessage, MessageToClient, MessageToServer } from "./types";

import { exists, readdir } from "fs-extra";
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { Worker } from "node:worker_threads";
import { WebSocket, WebSocketServer } from "ws";

class Server {
    constructor(private ws: WebSocket) {
        ws.on("message", this.onMessage.bind(this));
    }

    private sendMessage<T extends MessageToClient>(message: T): void {
        this.ws.send(JSON.stringify(message));
    }

    private async onMessage(data: WebSocket.RawData, _isBinary: boolean) {
        let r: MessageToServer = null!;

        try {
            r = JSON.parse(data.toString());

            if (!("type" in r)) {
                throw new Error("Invalid message (does not contain .type)");
            }

            switch (r.type) {
                case "queryBundles": {
                    const p = (await readdir(BUILDS_PATH, { withFileTypes: true }))
                        .filter((dir) => dir.isDirectory())
                        .map(async (dir) => {
                            const p = join(BUILDS_PATH, dir.name, "info.json");

                            if (!await exists(p)) {
                                return;
                            }

                            const text = await readFile(p, "utf8");

                            return JSON.parse(text) as BundleInfo;
                        });

                    const bundles = (await Promise.all(p)).filter((x) => x != null);

                    this.sendMessage<BundlesResponseMessage>({
                        type: "queryBundlesResponse",
                        bundles,
                    });
                    break;
                }
                case "getAllBundleFiles": {
                    // prevent path traversal
                    assert(r.bundleHash.match(/^[a-z0-9]+?$/i), "invalid bundleHash");

                    const bundlePath = join(BUILDS_PATH, r.bundleHash, ".modules");
                    const moduleFiles = await Promise.all((await readdir(bundlePath)).map(async (fileName) => [fileName.substring(0, fileName.length - 3), await readFile(join(bundlePath, fileName), "utf8")] as const));

                    this.sendMessage<AllBundleFilesResponseMessage>({
                        type: "getAllBundleFilesResponse",
                        bundleHash: r.bundleHash,
                        files: Object.fromEntries(moduleFiles),
                    });
                    break;
                }
                case "getBundleFile": {
                    // prevent path traversal
                    assert(r.bundleHash.match(/^[a-z0-9]+?$/i), "invalid bundleHash");
                    assert(r.moduleNumber.match(/^[0-9]+?$/i), "invalid moduleNumber");

                    const filePath = join(BUILDS_PATH, r.bundleHash, ".modules", `${r.moduleNumber}.js`);

                    this.sendMessage<BundleFileResponseMessage>({
                        type: "getBundleFileResponse",
                        bundleHash: r.bundleHash,
                        moduleNumber: r.moduleNumber,
                        fileText: await readFile(filePath, "utf8"),
                    });

                    break;
                }
                case "getBundleMetadata": {
                    // prevent path traversal
                    assert(r.bundleHash.match(/^[a-z0-9]+?$/i), "invalid bundleHash");

                    const p = join(BUILDS_PATH, r.bundleHash, "info.json");

                    this.sendMessage<BundleMetadataResponseMessage>({
                        type: "getBundleMetadataResponse",
                        bundleHash: r.bundleHash,
                        metadata: JSON.parse(await readFile(p, "utf8")) as BundleInfo,
                    });

                    break;
                }
                case "getBundleDepGraph": {
                    // prevent path traversal
                    assert(r.bundleHash.match(/^[a-z0-9]+?$/i), "invalid bundleHash");

                    const p = join(BUILDS_PATH, r.bundleHash, "deps.json");

                    this.sendMessage<BundleDepGraphResponseMessage>({
                        type: "getBundleDepGraphResponse",
                        bundleHash: r.bundleHash,
                        depGraph: JSON.parse(await readFile(p, "utf8")) as DepsJson,
                    });
                    break;
                }
                default:
                    // @ts-expect-error r.type should be `never` error because all cases are handled
                    error(`unexpected message type: ${r.type}`);
            }
        } catch (e) {
            this.sendMessage<ErrorMessage>({
                type: "error",
                sourceType: r?.type ?? "unknown",
                message: (e as Error).message,
            });
        }
    }
}

(async function () {
    if (!existsSync(BUILDS_PATH)) {
        await mkdir(BUILDS_PATH);
    }

    // microsoft/typescript#58561 insane "bug"
    // @ts-expect-error ^^
    const _watcherWorker = new Worker(new URL("./watcher.ts", import.meta.url));
    const wss = new WebSocketServer({ port: 6767 });

    wss.on("connection", (ws: WebSocket) => {
        new Server(ws);
    });
})();

