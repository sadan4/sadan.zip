#!/usr/bin/env node
import { CircularDependencyRspackPlugin, type ExternalItemFunctionData, type ExternalItemValue, optimize, rspack, SourceMapDevToolPlugin } from "@rspack/core";

import { move } from "fs-extra";
import { rm } from "node:fs/promises";
import { builtinModules } from "node:module";
import { join, relative, resolve } from "node:path";

const { dirname } = import.meta;
const isWindowsOnArm = process.platform === "win32" && process.arch === "arm64";
const projectRoot = resolve(dirname, "..");
const ssrRoot = join(projectRoot, "dist", "server");
const ssrJsDir = join(ssrRoot, "j");
const ssrEntry = join(ssrRoot, "index.js");
const ssrEntryMap = join(ssrRoot, "index.js.map");
const ssrTempOutput = join(ssrRoot, "index.temp.js");
const ssrTempOutputMap = join(ssrRoot, "index.temp.js.map");

function main() {
    if (isWindowsOnArm) {
        console.warn("Not minifying SSR bundle because cloudflare vite plugin is not supported on this platform");
        return;
    }

    const compiler = rspack({
        entry: ssrEntry,
        output: {
            module: true,
            chunkFormat: "module",
            chunkLoading: "import",
            library: {
                type: "module",
            },
            filename: "server/index.temp.js",
        },
        optimization: {
            moduleIds: "natural",
            mangleExports: "size",
        },
        plugins: [
            new optimize.LimitChunkCountPlugin({
                maxChunks: 1,
            }),
            new SourceMapDevToolPlugin({
                filename: "[file].map[query]",
                append: "\n//# sourceMappingURL=[url]",
            }),
            new CircularDependencyRspackPlugin({
                failOnError: true,
                exclude: /node_modules/,
            }),
        ],
        module: {
            rules: [
                {
                    test: /\.m?js$/,
                    extractSourceMap: true,
                    loader: "builtin:swc-loader",
                    options: {
                        jsc: {
                            parser: {
                                syntax: "ecmascript",
                                explicitResourceManagement: true,
                            },
                            // needed so swc doesn't down-level all syntax
                            target: "es2024",
                        },
                    },
                },
            ],
        },
        target: "es2024",
        async externals(ctx: ExternalItemFunctionData): Promise<ExternalItemValue | void> {
            if (ctx.request && /\.wasm$/.test(ctx.request)) {
                // @ts-expect-error rspack has really bad types
                const fullPath = await ctx.getResolve!()(ctx.context, ctx.request)!;
                const relativeToRoot = `./${relative(ssrRoot, fullPath)}`;

                return `module ${relativeToRoot}`;
            // NOTE: rspack uses createRequire instead of actual imports for some fucking reason
            } else if (ctx.request) {
                if (ctx.request.startsWith("node:")) {
                    return `module ${ctx.request}`;
                } else if (builtinModules.includes(ctx.request)) {
                    return `module node:${ctx.request}`;
                }
            }
        },
    });

    compiler.run(async (err, stats) => {
        if (err) {
            throw err;
        }


        if (stats?.hasErrors()) {
            console.log(stats.toString("errors-warnings"));
            throw new Error("SSR minification failed");
        }

        console.log(stats?.toString("normal"));

        await move(ssrTempOutput, ssrEntry, { overwrite: true });
        await move(ssrTempOutputMap, ssrEntryMap, { overwrite: true });
        await rm(ssrJsDir, { recursive: true });


        compiler.close((err) => {
            if (err) {
                throw err;
            }
            resolve();
        });
    });
}

await main();

export { };
