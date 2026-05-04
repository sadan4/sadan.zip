import { defineConfig } from "@rsbuild/core";
import { tanstackStart } from "@tanstack/react-start/plugin/rsbuild";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginBabel } from "@rsbuild/plugin-babel";
import type { ExternalItemFunctionData, ExternalItemValue } from "@rspack/core";
import { builtinModules } from "node:module";

export default defineConfig(async ({ }) => {
    Error.stackTraceLimit = 999
    return {
        plugins: [
            pluginReact({ splitChunks: false }),
            pluginBabel({
                include: /\.[jt]sx?$/,
                exclude: [/[\\/]node_modules[\\/]/],
                babelLoaderOptions(opts) {
                    opts.plugins?.unshift([
                        'babel-plugin-react-compiler',
                        {},
                    ]);
                }
            }),
            tanstackStart({
                router: {
                    quoteStyle: "double",
                    semicolons: true,
                },
                prerender: {
                    enabled: true,
                    failOnError: false,
                }
            }),
        ],
        output: {
            externals(data: ExternalItemFunctionData): ExternalItemValue | undefined {
                let id = data.request;
                if (id?.startsWith("node:")) {
                    return true;
                } else if (builtinModules.includes(id!)) {
                    return true;
                } 
            }
        }
    }
});