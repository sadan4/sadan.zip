import { assert, error } from "@/utils/error";

import { BUILDS_PATH } from "./constants";
import { migrateIfNeeded } from "./migration";
import { type AllBundleFilesResponseMessage, type BaseMessageToClient, type BundleDepGraphResponseMessage, type BundleFileResponseMessage, BundleInfo, type BundleMetadataResponseMessage, type BundlesResponseMessage, type DepsJson, MessageToClient, MessageToServer } from "./types";

import { exists, readdir } from "fs-extra";
import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { Worker } from "node:worker_threads";
import { WebSocket, WebSocketServer } from "ws";
import z from "zod";

class Server {
    static #VERIFY_OUTGOING_MESSAGES = true;
    #ws: WebSocket;

    constructor(ws: WebSocket) {
        this.#ws = ws;
        this.#ws.on("message", this.#onMessage.bind(this));
    }

    #sendMessage<T extends MessageToClient>(message: T): void {
        if (Server.#VERIFY_OUTGOING_MESSAGES) {
            MessageToClient.parse(message);
        }
        this.#ws.send(JSON.stringify(message));
    }

    async #onMessage(data: WebSocket.RawData, _isBinary: boolean) {
        let _r: any;

        try {
            _r = JSON.parse(data.toString());

            const r = _r = MessageToServer.parse(_r);

            const reply = <T extends BaseMessageToClient>(message: Omit<T, "messageId">) => {
                this.#sendMessage({
                    messageId: r.messageId,
                    ...message,
                } as any as MessageToClient);
            };

            console.debug("got message", { r });

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

                            return BundleInfo.parse(JSON.parse(text));
                        });

                    const bundles = (await Promise.all(p)).filter((x) => x != null);

                    reply<BundlesResponseMessage>({
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

                    reply<AllBundleFilesResponseMessage>({
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

                    reply<BundleFileResponseMessage>({
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

                    reply<BundleMetadataResponseMessage>({
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

                    reply<BundleDepGraphResponseMessage>({
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
        } catch (e: any) {
            let message: string;

            if (e instanceof z.ZodError) {
                message = z.prettifyError(e);
            } else if (e?.message) {
                message = String(e.message);
            } else {
                message = String(e);
            }

            this.#sendMessage({
                type: "error",
                messageId: _r?.messageId ?? -1,
                message,
            });
        }
    }
}

(async function () {
    if (!existsSync(BUILDS_PATH)) {
        await mkdir(BUILDS_PATH);
    }

    await migrateIfNeeded();

    // microsoft/typescript#58561 insane "bug"
    // @ts-expect-error ^^
    const _watcherWorker = new Worker(new URL("./watcher.ts", import.meta.url));
    const wss = new WebSocketServer({ port: 8044 });

    console.log("WebSocket server started on port 8044");

    wss.on("connection", (ws: WebSocket) => {
        console.log("got new connection");
        new Server(ws);
    });
})();

