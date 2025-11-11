/// <reference types="vitest/config" />
import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
import path, { join } from "node:path";
import { fileURLToPath } from "node:url";
import { generate } from "rollup-plugin-generate";
import { defineConfig } from "vite";
import devtoolsJson from "vite-plugin-devtools-json";
import tsconfigPaths from "vite-tsconfig-paths";

const dirname = typeof __dirname !== "undefined" ? __dirname : path.dirname(fileURLToPath(import.meta.url));

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig({
    plugins: [
        react({
            babel: {
                plugins: ["babel-plugin-react-compiler"],
            },
        }),
        tailwindcss(),
        tsconfigPaths(),
        devtoolsJson(),
        generate({ emitDts: true }),
    ],
    build: {
        sourcemap: true,
        // top-level await in esm
        target: "es2022",
        emptyOutDir: false,
        rollupOptions: {
            output: {
                chunkFileNames: "js/[hash].js",
                entryFileNames: "js/[hash].js",
                assetFileNames: "assets/[hash].[ext]",
            },
        },
    },
    css: {
        modules: {
            localsConvention: "camelCaseOnly",
            generateScopedName: "[local]_[contentHash:5]",
        },
        preprocessorOptions: {
            scss: {
                loadPaths: [join(dirname, "src", "styles")],
            },
        },
    },
    test: {
        projects: [
            {
                extends: true,
                plugins: [
                    // The plugin will run tests for the stories defined in your Storybook config
                    // See options at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon#storybooktest
                    storybookTest({
                        configDir: path.join(dirname, ".storybook"),
                    }),
                ],
                test: {
                    name: "storybook",
                    browser: {
                        enabled: true,
                        headless: true,
                        provider: "playwright",
                        instances: [
                            {
                                browser: "chromium",
                            },
                        ],
                    },
                    setupFiles: [".storybook/vitest.setup.ts"],
                },
            },
        ],
    },
});
