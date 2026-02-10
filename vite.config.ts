import { cloudflare } from "@cloudflare/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { devtools } from "@tanstack/devtools-vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";

import { monacoEditor } from "./scripts/vite-plugin-monaco-editor";

import { fileURLToPath, URL } from "url";

import { join } from "node:path";
import { generate } from "rollup-plugin-generate";
import { defineConfig } from "vite";
import devtoolsJSON from "vite-plugin-devtools-json";
import inspect from "vite-plugin-inspect";
import viteTsConfigPaths from "vite-tsconfig-paths";

const config = defineConfig(({ command, isSsrBuild }) => ({
    resolve: {
        alias: {
            "@": fileURLToPath(new URL("./src", import.meta.url)),
        },
    },
    plugins: [
        devtoolsJSON(),
        monacoEditor({
            languages: ["typescript", "javascript"],
            features: [],
        }),
        devtools(),
        // this is the plugin that enables path aliases
        viteTsConfigPaths({
            projects: ["./tsconfig.json"],
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
        viteReact({
            babel: {
                plugins: ["babel-plugin-react-compiler"],
            },
        }),
        cloudflare({
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
        // incompatible with netlify plugin
        !process.env.CI && inspect({
            build: true,
            // breaks on dev
            dev: false,
        }),
    ],
    build: {
        manifest: !process.env.CI,
        ssrManifest: !process.env.CI,
        ssrEmitAssets: true,
        target: "es2022",
        rolldownOptions: {
            output: {
                assetFileNames: "a/[hash:16].[ext]",
                chunkFileNames: "j/[hash:16].js",
                inlineDynamicImports: isSsrBuild || undefined,
            },
        },
        cssMinify: "lightningcss",
        sourcemap: true,
    },
    css: {
        // I would like to use lightningcss, but it doesn not support localsConvention
        // SEE: parcel-bundler/lightningcss#633
        transformer: "postcss",
        modules: {
            localsConvention: "camelCaseOnly",
            generateScopedName: command === "serve" ? "[local]__[hash:base64:8]" : "[hash:base64:8]",
        },
        preprocessorOptions: {
            scss: {
                loadPaths: [join(import.meta.dirname, "src", "styles")],
            },
        },
        devSourcemap: true,
    },
}));

export default config;
