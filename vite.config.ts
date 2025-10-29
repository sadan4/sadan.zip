/// <reference types="vitest/config" />
import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
import path, { join } from "node:path";
import { fileURLToPath } from "node:url";
import { type BuildEnvironmentOptions, defineConfig } from "vite";
import devtoolsJson from "vite-plugin-devtools-json";
import tsconfigPaths from "vite-tsconfig-paths";

const dirname = typeof __dirname !== "undefined" ? __dirname : path.dirname(fileURLToPath(import.meta.url));

const ssrBuildConfig: BuildEnvironmentOptions = {
    sourcemap: true,
    outDir: join(dirname, "dist", "server"),
    ssr: true,
    ssrEmitAssets: true,
    copyPublicDir: false,
    emptyOutDir: true,
    rollupOptions: {
        input: join(dirname, "src", "server.tsx"),
        output: {
            chunkFileNames: "js/[name]-[hash].js",
            entryFileNames: "[name].js",
            assetFileNames: "assets/[name]-[hash].[ext]",
        },
    },
};


const clientBuildConfig: BuildEnvironmentOptions = {
    sourcemap: true,
    // top-level await in esm
    target: "es2022",
    outDir: join(dirname, "dist", "client"),
    emitAssets: true,
    copyPublicDir: true,
    emptyOutDir: true,
    rollupOptions: {
        input: join(dirname, "src", "client.tsx"),
        output: {
            chunkFileNames: "js/[hash].js",
            entryFileNames: "[name].js",
            assetFileNames: "assets/[hash].[ext]",
        },
    },
};

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig(({ isSsrBuild }) => ({
    plugins: [
        tanstackRouter({
            target: "react",
            autoCodeSplitting: true,
            quoteStyle: "double",
        }) as any,
        react({
            babel: {
                plugins: [
                    [
                        "babel-plugin-react-compiler",
                        {
                            logger: {
                                logEvent(filename: any, event: {
                                    kind: string;
                                    detail: {
                                        reason: any;
                                        description: any;
                                        loc: {
                                            start: {
                                                line: any;
                                                column: any;
                                            };
                                        };
                                        suggestions: any;
                                    };
                                }) {
                                    return;
                                    if (event.kind === "CompileError") {
                                        console.error(`\nCompilation failed: ${filename}`);
                                        console.error(`Reason: ${event.detail.reason}`);
                                        if (event.detail.description) {
                                            console.error(`Details: ${event.detail.description}`);
                                        }
                                        if (event.detail.loc) {
                                            const {
                                                line,
                                                column,
                                            } = event.detail.loc.start ?? {};

                                            console.error(`Location: Line ${line}, Column ${column}`);
                                        }
                                        if (event.detail.suggestions) {
                                            console.error("Suggestions:", event.detail.suggestions);
                                        }
                                    }
                                },
                            },
                        },
                    ],
                ],
            },
        }),
        tailwindcss(),
        tsconfigPaths(),
        devtoolsJson(),
    ],
    build: isSsrBuild ? ssrBuildConfig : clientBuildConfig,
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
}));
