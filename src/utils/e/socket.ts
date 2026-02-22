import { type BaseMessageToServer, type MessageToClient, messageToClientSchema, type MessageToServer, messageToServerSchema } from "../../../server/types";
import { withResolvers } from "../async";

import z from "zod";

let _pendingWs: Promise<WebSocket> | null = null;
let _ws: WebSocket | null = null;
const WS_URL = "wss://s-d-br.sadan.zip";

function ensureConnection() {
    if (_ws?.readyState === WebSocket.OPEN) {
        return Promise.resolve(_ws);
    }
    if (!_pendingWs) {
        _ws = new WebSocket(WS_URL);

        const { promise, resolve, reject } = withResolvers<WebSocket>();

        _ws.addEventListener("open", () => {
            resolve(_ws!);
        }, { once: true });
        setTimeout(() => reject(new Error("Timeout while connecting")), 10_000);

        _pendingWs = promise;
    }
    return _pendingWs;
}

type Discriminate<U extends { type: string; }, K extends U["type"]> = U extends { type: K; } ? U : never;

let NEXT_ID = 1;

export async function sendMessage<T extends MessageToClient["type"] = never>(msg: BaseMessageToServer): Promise<Discriminate<MessageToClient, T>> {
    if ("messageId" in msg) {
        throw new Error("messageId is automatically assigned and should not be included in the message");
    }

    const id = NEXT_ID++;

    (msg as MessageToServer).messageId = id;
    try {
        messageToServerSchema.parse(msg);
    } catch (e) {
        if (e instanceof z.ZodError) {
            throw new Error(`Invalid message format: ${z.prettifyError(e)}`);
        } else {
            throw e;
        }
    }

    const ws = await ensureConnection();
    const { promise, resolve, reject } = withResolvers<Discriminate<MessageToClient, T>>();
    const abort = new AbortController();
    const { signal } = abort;

    ws.addEventListener("close", (ev) => {
        // Invalidate cached WebSocket so future calls can establish a new connection
        _ws = null;
        reject(new Error(`Connect Closed: ${ev.reason}`));
        abort.abort();
    }, { signal });

    ws.addEventListener("error", (ev) => {
        // Invalidate cached WebSocket so future calls can establish a new connection
        _ws = null;
        reject(new Error("Socket Error", { cause: ev }));
        abort.abort();
    }, { signal });

    ws.addEventListener("message", (ev: MessageEvent<string>) => {
        try {
            const _data = JSON.parse(ev.data);
            const msg = messageToClientSchema.parse(_data);

            if (msg.messageId !== id) {
                return;
            }
            if (msg.type === "error") {
                reject(new Error(msg.message));
                abort.abort();
                return;
            }
            resolve(msg as Discriminate<MessageToClient, T>);
        } catch (e) {
            if (e instanceof SyntaxError) {
                reject(new Error("Failed to parse message from server", { cause: e }));
            } else if (e instanceof z.ZodError) {
                reject(new Error(`Invalid message received from server: ${z.prettifyError(e)}`, { cause: e }));
            } else {
                reject(new Error("Unexpected error while handling message from server", { cause: e }));
            }
        }
        abort.abort();
    }, { signal });

    ws.send(JSON.stringify(msg));

    setTimeout(() => {
        reject(new Error("Timeout while waiting for response"));
        abort.abort();
    }, 40_000);

    return promise;
}
