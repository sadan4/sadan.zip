// @ts-check
import type { BuildOptions } from "esbuild";
import { join } from "node:path";

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
