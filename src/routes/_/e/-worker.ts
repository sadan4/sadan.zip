import { default as initWasm, get_builds_page } from "@sadan4/libsadancore";

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

export interface BuildsPage {
    builds: Meta[];
    /**
     * pass as `before` to get the next page, `undefined` on the last page
     */
    nextCursor: bigint | undefined;
}

/**
 * @param before only get builds first seen before this, from {@link BuildsPage.nextCursor}
 */
async function getBuildsPage(before?: bigint): Promise<BuildsPage> {
    await initWasm();

    const page = await get_builds_page(before);

    const builds = page.builds.map((meta) => {
        const { build_hash, build_number, channels, entry_point, first_seen } = meta;

        meta.free();
        return {
            build_hash,
            build_number,
            channels: channels as Channel[],
            entry_point,
            first_seen,
        };
    });

    const nextCursor = page.next_cursor;

    page.free();
    return {
        builds,
        nextCursor,
    };
}

export type GetBuildsPageFn = typeof getBuildsPage;

self.onconnect = function ({ ports: [port] }) {
    comlink.expose(getBuildsPage, port);
};
