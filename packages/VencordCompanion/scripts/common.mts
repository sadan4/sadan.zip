// @ts-check
import { join } from "node:path";

import type { BuildOptions } from "esbuild";

const __dirname = import.meta.dirname;
const rootDir = join(__dirname, "..");

export const commonOpts = {
    entryPoints: ["./src/extension.ts"],
    minify: true,
    treeShaking: true,
    bundle: true,
    external: ["vscode"],
    platform: "node",
    sourcemap: "inline",
    logLevel: "info",
    tsconfig: join(rootDir, "tsconfig.json"),
    outfile: "dist/extension.js",
} satisfies BuildOptions;
