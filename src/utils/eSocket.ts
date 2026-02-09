import type { MessageToClient, MessageToServer } from "../../server/types";

let _ws: WebSocket | null = null;
const WS_URL = "ws://localhost:6767";

async function ensureConnection() {
    if (!_ws) {
        _ws = new WebSocket(WS_URL);

        const { promise, resolve, reject } = Promise.withResolvers<WebSocket>();

        _ws.addEventListener("open", () => {
            resolve(_ws!);
        }, { once: true });
        setTimeout(() => reject(new Error("Timeout while connecting")), 10_000);
        return promise;
    }
    return Promise.resolve(_ws);
}

type Discriminate<U extends { type: string; }, K extends U["type"]> = U extends { type: K; } ? U : never;

async function sendMessage<T extends MessageToClient["type"]>(msg: MessageToServer): Promise<Discriminate<MessageToClient, T>> {
    return await null!;
}

const g = await sendMessage<"queryBundlesResponse">(null!);

export async function getAvailableBuilds() {
    const ws = await ensureConnection();
}
