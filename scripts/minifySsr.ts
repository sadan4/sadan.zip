#!/usr/bin/env node
import { copy, move } from "fs-extra";
import { rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { build } from "rolldown";

const { dirname } = import.meta;
const isWindowsOnArm = process.platform === "win32" && process.arch === "arm64";
const projectRoot = resolve(dirname, "..");
const ssrRoot = join(projectRoot, "dist", "server");
const ssrJsDir = join(ssrRoot, "j");
const ssrEntry = join(ssrRoot, "index.js");
const ssrEntryMap = join(ssrRoot, "index.js.map");
const ssrTempOutput = join(ssrRoot, "index.temp.js");
const ssrTempOutputMap = join(ssrRoot, "index.temp.js.map");

async function main() {
    if (isWindowsOnArm) {
        console.warn("Not minifying SSR bundle because cloudflare vite plugin is not supported on this platform");
        return;
    }

    await build({
        input: ssrEntry,
        output: {
            file: ssrTempOutput,
            codeSplitting: false,
            minify: {
                codegen: {
                    legalComments: "none",
                    removeWhitespace: true,
                },
                compress: {
                    dropConsole: true,
                },
                mangle: {
                    toplevel: true,
                },
            },
            minifyInternalExports: true,
            comments: false,
            sourcemap: true,
        },
        external: [/\.wasm$/, /^node:/],
    });

    await move(ssrTempOutput, ssrEntry, { overwrite: true });
    // copy instead of move so that either resolving via appending `.map` to the filename or
    // reading the `sourceMappingURL` comment will work
    await copy(ssrTempOutputMap, ssrEntryMap, { overwrite: true });
    await rm(ssrJsDir, { recursive: true });
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
