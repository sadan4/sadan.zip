import type { Thenable } from "@/utils/types";

import { BUILDS_PATH } from "./constants";
import { BundleInfo } from "./types";

import { exists } from "fs-extra";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

enum Version {
    V0,
    /**
     * {@link BundleInfo.entryPoint|entryPoint} field added to {@link BundleInfo}
     */
    V1,
}

const CURRENT_VERSION = Version.V1;
const VERSION_FILE = join(BUILDS_PATH, ".ver");

const migrations = Object.freeze({
    [Version.V0]() {
    },
    async [Version.V1]() {
        const bundleInfoMaybeEntryPoint = BundleInfo.partial({ entryPoint: true });

        for (const entry of await readdir(BUILDS_PATH, { withFileTypes: true })) {
            if (!entry.isDirectory()) {
                continue;
            }

            const entryPath = resolve(entry.parentPath, entry.name);
            const infoPath = join(entryPath, "info.json");

            if (!await exists(infoPath)) {
                continue;
            }

            const info = bundleInfoMaybeEntryPoint.parse(JSON.parse(await readFile(infoPath, "utf8")));

            // we don't need to do anything
            if (info.entryPoint !== undefined) {
                continue;
            }

            info.entryPoint = null;

            await writeFile(infoPath, JSON.stringify(info, null, 2), "utf8");
        }
    },
} satisfies Record<Version, () => Thenable<void>>);

async function writeVersion(version: Version): Promise<void> {
    return await writeFile(VERSION_FILE, `${version}`, "utf8");
}

async function readVersion(): Promise<Version> {
    await mkdir(dirname(VERSION_FILE), { recursive: true });
    if (!await exists(VERSION_FILE)) {
        await writeVersion(CURRENT_VERSION);
        return CURRENT_VERSION;
    }

    const version = +(await readFile(VERSION_FILE, "utf8")).trim();

    switch (version) {
        case Version.V0:
        case Version.V1:
            return version;
        default:
            throw new Error(`Unsupported version: ${version}. Latest supported version is ${CURRENT_VERSION}.`);
    }
}

export async function migrateIfNeeded() {
    const versionOnDisk = await readVersion();

    if (versionOnDisk < CURRENT_VERSION) {
        console.info(`Migrating from version ${versionOnDisk} to ${CURRENT_VERSION}...`);
        for (let v = versionOnDisk + 1; v <= CURRENT_VERSION; ++v) {
            console.info("starting migration for version: ", v);
            try {
                await migrations[v as Version]();
                await writeVersion(v);
            } catch (err) {
                throw new Error(`Failed to migrate to version ${v}`, { cause: err });
            }
            console.info("finished migration for version: ", v);
        }
        console.info(`Migration from ${versionOnDisk} to ${CURRENT_VERSION} complete.`);
    } else {
        console.info(`No migration needed. Current version on disk is ${versionOnDisk}.`);
    }
}
