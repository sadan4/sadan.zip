import { BUILD_METADATA_URL } from "@/utils/constants";
import type { TBundleHash } from "@/utils/types";
import { decode } from "@msgpack/msgpack";

import { zstdDecompressSync } from "node:zlib";
import z from "zod";

const BuildMeta = z.object({
    buildNumber: z.number(),
    /**
     * unix timestamp in ms
     */
    firstSeen: z.number(),
    channel: z.array(z.enum(["Stable", "Canary"])).catch([]),
});

export type BuildMeta = z.infer<typeof BuildMeta>;

/**
 * don't hold up the embed for too long if the build server is slow
 */
const FETCH_TIMEOUT_MS = 2000;

/**
 * @returns null if the build server doesn't have the build or can't be reached
 */
export async function fetchBuildMeta(buildHash: TBundleHash): Promise<BuildMeta | null> {
    try {
        const rsp = await fetch(BUILD_METADATA_URL(buildHash), {
            signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
        });

        if (!rsp.ok) {
            return null;
        }

        // zstd compressed msgpack
        const raw = decode(zstdDecompressSync(await rsp.arrayBuffer()), { useBigInt64: false });

        return BuildMeta.parse(raw);
    } catch (e) {
        console.error(`Failed to fetch build metadata for ${buildHash}`, e);
        return null;
    }
}
