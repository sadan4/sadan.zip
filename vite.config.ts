import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

import { defineConfig } from "vite";
import devtoolsJson from "vite-plugin-devtools-json";
import tsconfigPaths from "vite-tsconfig-paths";

// https://vite.dev/config/
export default defineConfig({
    plugins: [
        react({
            babel: {
                plugins: [
                    [
                        "babel-plugin-react-compiler",
                        {
                            logger: {
                                logEvent(filename: any, event: { kind: string;
                                    detail: { reason: any;
                                        description: any;
                                        loc: { start: { line: any;
                                            column: any; }; };
                                        suggestions: any; }; }) {
                                    if (event.kind === "CompileError") {
                                        console.error(`\nCompilation failed: ${filename}`);
                                        console.error(`Reason: ${event.detail.reason}`);

                                        if (event.detail.description) {
                                            console.error(`Details: ${event.detail.description}`);
                                        }

                                        if (event.detail.loc) {
                                            const { line, column } = event.detail.loc.start ?? {};

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
    },
    css: {
        modules: {
            localsConvention: "camelCase",
        },
    },
});
