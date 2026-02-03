import netlify from "@netlify/vite-plugin-tanstack-start";
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

const config = defineConfig(({ command }) => ({
    resolve: {
        alias: {
            "@": fileURLToPath(new URL("./src", import.meta.url)),
        },
    },
    plugins: [
        devtoolsJSON(),
        monacoEditor({
            languages: ["typescript"],
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
        // NOTE: bug in netlify on windows netlify/primitives#408
        command !== "serve" && netlify(),
        // incompatible with netlify plugin
        !process.env.CI && inspect({
            build: true,
        }),
    ],
    build: {
        manifest: !process.env.CI,
        ssrManifest: !process.env.CI,
        target: "es2022",
        rolldownOptions: {
            output: {
                assetFileNames: "a/[hash:16].[ext]",
                chunkFileNames: "j/[hash:16].js",
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
