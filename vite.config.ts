import babel from "@rolldown/plugin-babel";
import tailwindcss from "@tailwindcss/vite";
import { devtools } from "@tanstack/devtools-vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact, { reactCompilerPreset } from "@vitejs/plugin-react";

import { monacoEditor } from "./scripts/vite-plugin-monaco-editor";

import { fileURLToPath } from "url";

import { dirname, join } from "node:path";
import { generate } from "rollup-plugin-generate";
import { defineConfig, type UserConfig } from "vite";
import devtoolsJSON from "vite-plugin-devtools-json";


const config = defineConfig(async ({ command, isSsrBuild }) => {
    const isWindowsOnArm = process.platform === "win32" && process.arch === "arm64";
    const __dirname = dirname(fileURLToPath(import.meta.url));
    const srcDir = join(__dirname, "src");
    const isDev = command === "serve";
    const isCi = !!process.env.CI;

    return {
        plugins: [
            devtoolsJSON(),
            monacoEditor({
                languages: ["typescript", "javascript"],
            }),
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
            generate({
                emitDts: true,
                cache: {
                    build: "filesystem",
                    watch: "filesystem",
                },
            }),
            viteReact(),
            babel({
                presets: [reactCompilerPreset()],
            }),
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
                    chunkFileNames: "j/[hash:16].js",
                    inlineDynamicImports: isSsrBuild || undefined,
                },
                optimization: {
                    inlineConst: {
                        mode: "all",
                        pass: 1,
                    },
                },
            },
            cssMinify: "lightningcss",
            sourcemap: true,
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
        experimental: {
        },
    } satisfies UserConfig;
});

export default config;
