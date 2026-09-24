import { default as initWasm, get_builds } from "@sadan4/libsadancore";

import * as comlink from "comlink";

const self = globalThis.self as any as SharedWorkerGlobalScope;

export type Channel = "stable" | "canary";

export interface Meta {
    build_hash: string;
    build_number: number;
    channels: Channel[];
    entry_point: number | undefined;
    first_seen: bigint;
}

async function getBuilds(): Promise<Meta[]> {
    await initWasm();
    return (await get_builds()).map(({ build_hash, build_number, channels, entry_point, first_seen }) => ({
        build_hash,
        build_number,
        channels: channels as Channel[],
        entry_point,
        first_seen,
    }));
}

export type GetBuildsFn = typeof getBuilds;

self.onconnect = function ({ ports: [port] }) {
    comlink.expose(getBuilds, port);
};
