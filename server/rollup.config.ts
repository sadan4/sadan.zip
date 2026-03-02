import cjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import swc from "@rollup/plugin-swc";
import omt from "@surma/rollup-plugin-off-main-thread";

import { basename } from "path";

import { defineConfig, type Plugin } from "rollup";
import { cleandir } from "rollup-plugin-cleandir";
import tsconfigPaths from "rollup-plugin-tsconfig-paths";

const NATIVE_MODULE_REGEX = /\.node$/;
const queryRE = /\?.*$/s;
const hashRE = /#.*$/s;

function cleanUrl(url: string): string {
    return url.replace(hashRE, "").replace(queryRE, "");
}

function copyNativeModules(): Plugin {
    return {
        name: "rollup-plugin-copy-native-modules",
        async load(id) {
            if (id[0] === "\0" || !NATIVE_MODULE_REGEX.test(id)) {
                return;
            }

            const assetId = this.emitFile({
                type: "asset",
                name: basename(id),
                source: await this.fs.readFile(cleanUrl(id)),
            });

            return /*js*/`
                import { createRequire } from "node:module";
                export default createRequire(import.meta.url)("./" + import.meta.ROLLUP_FILE_URL_${assetId});
            `;
        },
    };
}

export default defineConfig({
    input: "server/index-native.ts",
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
        file: "dist.server/index.js",
        sourcemap: "inline",
    },
});
