import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
import path, { join } from "node:path";
import { fileURLToPath } from "node:url";
import { generate } from "rollup-plugin-generate";
import { type BuildEnvironmentOptions, defineConfig } from "vite";
import devtoolsJson from "vite-plugin-devtools-json";
import inspect from "vite-plugin-inspect";
import tsconfigPaths from "vite-tsconfig-paths";

const dirname = typeof __dirname !== "undefined" ? __dirname : path.dirname(fileURLToPath(import.meta.url));

const ssrBuildConfig: BuildEnvironmentOptions = {
    sourcemap: true,
    outDir: join(dirname, "dist", "server"),
    ssr: true,
    ssrEmitAssets: true,
    copyPublicDir: false,
    emptyOutDir: true,
    cssCodeSplit: false,
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
    manifest: true,
    cssCodeSplit: false,
    rollupOptions: {
        input: join(dirname, "src", "client.tsx"),
        output: {
            chunkFileNames: "js/[name]-[hash].js",
            entryFileNames: "[name].js",
            assetFileNames: "assets/[name]-[hash].[ext]",
            // chunkFileNames: "js/[hash].js",
            // entryFileNames: "js/[hash].js",
            // assetFileNames: "assets/[hash].[ext]",
        },
    },
};

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig(({ isSsrBuild }) => {
    return {
        plugins: [
            tanstackRouter({
                target: "react",
                autoCodeSplitting: true,
                quoteStyle: "double",
            }),
            react({
                babel: {
                    plugins: ["babel-plugin-react-compiler"],
                },
            }),
            tailwindcss(),
            tsconfigPaths(),
            devtoolsJson(),
            generate({ emitDts: true }),
            !process.env.STORYBOOK && !process.env.CI && inspect({}),
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
    };
});
