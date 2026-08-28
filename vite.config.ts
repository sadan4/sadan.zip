import tailwindcss from "@tailwindcss/vite";
import { devtools } from "@tanstack/devtools-vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";

import { omt } from "./scripts/vite-plugin-omt.ts";

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Rollup, type UserConfig } from "vite";
import devtoolsJSON from "vite-plugin-devtools-json";

const groups: Rollup.CodeSplittingGroup[] = [
    {
        test: /node_modules\/(?:react|react-dom)/,
        name: "c-react",
    },
    {
        test: /node_modules\/typescript/,
        name: "c-ts",
    },
    {
        test: /node_modules\/zod/,
        name: "c-zod",
    },
    {
        test: /node_modules\/lucide/,
        name: "c-icons",
    },
    {
        test: /node_modules\/zustand/,
        name: "c-zustand",
    },
    {
        test: /node_modules\/(?:@vencord-companion|acorn|ts-api-utils|@sadan4\/devtools-pretty-printer)/,
        name: "c-dbp",
    },
];

const config = defineConfig(async ({ command, isSsrBuild }) => {
    const isWindowsOnArm = process.platform === "win32" && process.arch === "arm64";
    const __dirname = dirname(fileURLToPath(import.meta.url));
    const srcDir = join(__dirname, "src");
    const isDev = command === "serve";
    const isCi = !!process.env.CI;

    return {
        plugins: [
            devtoolsJSON(),
            devtools({
                consolePiping: {
                    enabled: true,
                    // don't pipe warn and error logs, piping hides stack traces and the source
                    levels: ["info", "log", "debug"],
                },
            }),
            tailwindcss(),
            tanstackStart({
                router: {
                    quoteStyle: "double",
                    semicolons: true,
                },
                prerender: {
                    enabled: true,
                    failOnError: false,
                },
            }),
            viteReact({
                compiler: true,
            }),
            omt(),
            !isWindowsOnArm && (await import("@cloudflare/vite-plugin")).cloudflare({
                viteEnvironment: {
                    name: "ssr",
                },
                config: {
                    observability: {
                        enabled: false,
                        head_sampling_rate: 1,
                        logs: {
                            enabled: true,
                            head_sampling_rate: 1,
                            persist: true,
                            invocation_logs: true,
                        },
                        traces: {
                            enabled: false,
                            persist: true,
                            head_sampling_rate: 1,
                        },
                    },
                },
            }),
        ],
        resolve: {
            alias: {
                "@": srcDir,
            },
            tsconfigPaths: true,
        },
        define: {
            IS_CLOUDFLARE: JSON.stringify(!isWindowsOnArm),
        },
        build: {
            manifest: !isCi,
            ssrManifest: !isCi,
            ssrEmitAssets: true,
            target: "es2022",
            rolldownOptions: {
                output: {
                    assetFileNames: "a/[hash:16].[ext]",
                    chunkFileNames(info) {
                        if (groups.findIndex((g) => g.name === info.name) !== -1) {
                            return "j/[name].[hash:16].js";
                        }
                        return "j/[hash:16].js";
                    },
                    inlineDynamicImports: isSsrBuild || undefined,
                    codeSplitting: {
                        groups,
                    },
                },
                experimental: {
                    lazyBarrel: true,
                    nativeMagicString: true,
                },
                optimization: {
                    inlineConst: {
                        mode: "smart",
                        pass: 1,
                    },
                },
            },
            cssMinify: "lightningcss",
            sourcemap: true,
        },
        worker: {
            format: "es",
        },
        json: {
            namedExports: false,
            stringify: true,
        },
        css: {
            // I would like to use lightningcss, but it doesn not support localsConvention
            // SEE: parcel-bundler/lightningcss#633
            transformer: "postcss",
            modules: {
                localsConvention: "camelCaseOnly",
                generateScopedName: isDev ? "[local]__[hash:base64:8]" : "[hash:base64:8]",
            },
            preprocessorOptions: {
                scss: {
                    loadPaths: [join(srcDir, "styles")],
                },
            },
            devSourcemap: true,
        },
        server: {
            watch: {
                ignored: ["**/target", "**/builds", "**/dist", "**/.direnv", "**/crates"],
            },
        },
        devtools: {
            // https://github.com/rolldown/rolldown/issues/5896
            // https://github.com/rolldown/rolldown/pull/9219
            enabled: false,
        },
        optimizeDeps: {
        },
        environments: {
            ssr: {
                optimizeDeps: {
                    // the dep scanner crawls the dynamic devtools import in
                    // src/components/Devtools.tsx even though the branch is dead in the
                    // ssr build. the devtools are solid based, and solid resolves to its
                    // server build under the worker condition, which is missing exports
                    // the devtools use, so prebundling them for ssr fails
                    exclude: [
                        "@tanstack/devtools",
                        "@tanstack/devtools-ui",
                        "@tanstack/react-devtools",
                        "@tanstack/react-router-devtools",
                    ],
                },
            },
        },
    } satisfies UserConfig;
});

export default config;
