/// <reference types="vitest/config" />
import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
import path, { join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import devtoolsJson from "vite-plugin-devtools-json";
import tsconfigPaths from "vite-tsconfig-paths";

const dirname = typeof __dirname !== "undefined" ? __dirname : path.dirname(fileURLToPath(import.meta.url));

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig({
    plugins: [
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
        {
            name: "log-server-requests",
            configureServer(server) {
                server.middlewares.use((req, _res, next) => {
                    console.log(`[Vite Dev Server] ${req.method} ${req.url}`);
                    next();
                });
            },
        },
    ],
    build: {
        sourcemap: true,
        // top-level await in esm
        target: "es2022",
        emptyOutDir: false,
        rollupOptions: {
            output: {
                chunkFileNames: "js/[name]-[hash].js",
                entryFileNames: "js/[hash].js",
                assetFileNames: "assets/[hash].[ext]",
                onlyExplicitManualChunks: true,
                manualChunks: (() => {
                    const chunks = {
                        hooks: ["@react-hook/resize-observer", "@use-gesture/react"],
                        reactRouter: ["react-router"],
                        reactSpring: [
                            "@react-spring/shared",
                            "@react-spring/web",
                        ],
                        zustand: ["zustand"],
                        demangler: ["@sadan4/demangler"],
                        ui: ["@radix-ui/react-popover"],
                        misc: [
                            "fast-deep-equal",
                            "scroll-into-view-if-needed",
                            "classnames",
                        ],
                        react: ["react", "react-dom"],
                    };

                    const revChunks = Object
                        .entries(chunks)
                        .flatMap(([chunkName, modules]) => {
                            return modules.map((module) => [`/${module}/`, chunkName] as const);
                        });

                    return (chunk, _info) => {
                        return revChunks.find(([module]) => chunk.includes(module))?.[1];
                    };
                })(),
            },
        },
    },
    css: {
        devSourcemap: true,
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
