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
                plugins: ["babel-plugin-react-compiler"],
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
