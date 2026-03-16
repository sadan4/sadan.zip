import cjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import swc from "@rollup/plugin-swc";
import omt from "@surma/rollup-plugin-off-main-thread";

import copyNativeModules from "../scripts/rollup-plugin-copy-native-modules.ts";

import { defineConfig } from "rollup";
import { cleandir } from "rollup-plugin-cleandir";
import tsconfigPaths from "rollup-plugin-tsconfig-paths";


export default defineConfig({
    input: ["server/index.ts", "server/index-native.ts"],
    plugins: [
        nodeResolve({
            extensions: [".ts", ".js", ".node"],
        }),
        copyNativeModules(),
        cjs(),
        swc(),
        omt(),
        tsconfigPaths(),
        cleandir(["dist.server"]),
    ],
    external: [/@vencord-companion\//, "jsdom"],
    output: {
        format: "esm",
        dir: "dist.server",
        sourcemap: "inline",
    },
});
