import { defineConfig, type RsbuildConfig } from "@rsbuild/core";
import { pluginBabel } from "@rsbuild/plugin-babel";
import { pluginNodePolyfill } from "@rsbuild/plugin-node-polyfill";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSass } from "@rsbuild/plugin-sass";
import { optimize } from "@rspack/core";
import tailwindPostCss from "@tailwindcss/postcss";
import { tanstackStart } from "@tanstack/react-start/plugin/rsbuild";

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export default defineConfig(({ envMode }) => {
    const isDev = envMode === "development";
    const __dirname = dirname(fileURLToPath(import.meta.url));
    const srcDir = join(__dirname, "src");

    return {
        plugins: [
            pluginNodePolyfill(),
            pluginReact({ splitChunks: false }),
            pluginBabel({
                include: /\.[jt]sx?$/,
                exclude: [/[\\/]node_modules[\\/]/],
                babelLoaderOptions(opts) {
                    opts.plugins?.unshift([
                        "babel-plugin-react-compiler",
                        {},
                    ]);
                },
            }),
            tanstackStart({
                router: {
                    quoteStyle: "double",
                    semicolons: true,
                },
                prerender: {
                    enabled: true,
                },
            }),
            pluginSass({
                sassLoaderOptions: {
                    sassOptions: {
                        loadPaths: [join(srcDir, "styles")],
                    },
                },
            }),
        ],
        output: {
            distPath: {
                css: "c",
                svg: "a",
                font: "a",
                html: "./",
                wasm: "j",
                image: "a",
                media: "a",
                assets: "a",
                favicon: "./",
                cssAsync: "c",
            },
            sourceMap: {
                extract: true,
                css: true,
            },
            cssModules: {
                exportLocalsConvention: "camelCaseOnly",
                namedExport: true,
                localIdentName: isDev ? "[local]__[hash:base64:6]" : "[hash:base64:6]",
            },
        },
        source: {
            assetsInclude: [/\.wasm$/],
        },
        tools: {
            postcss: {
                postcssOptions: {
                    plugins: [tailwindPostCss()],
                },
            },
            rspack: {
                experiments: {
                    nativeWatcher: true,
                },
                module: {
                    rules: [
                        {
                            resourceQuery: /\?raw$/,
                            type: "asset/source",
                        },
                        {
                            test: /\.(?:js|mjs|ts)$/,
                            use: [
                                {
                                    loader: "builtin:swc-loader",
                                    options: {
                                        detectSyntax: "auto",
                                        collectTypeScriptInfo: {
                                            exportedEnum: !isDev,
                                        },
                                    },
                                },
                            ],
                        },
                    ],
                },
                optimization: {
                    concatenateModules: true,

                    splitChunks: {
                        cacheGroups: {
                            c: {
                                test: /src\/index\.css/,
                                chunks: "all",
                                minSize: 0,
                                name: "c",
                                priority: 10_000,
                            },
                        },
                    },
                },
            },
        },
        performance: {
            buildCache: true,
        },
        dev: {
            lazyCompilation: true,
        },
        environments: {
            client: {
                source: {
                    define: {
                        "import.meta.env.SSR": "false",
                        IS_CLOUDFLARE: "false",
                    },
                },
                output: {
                    sourceMap: {
                        js: "source-map",
                    },
                    distPath: {
                        js: "j",
                        jsAsync: "j",
                    },
                },
            },
            ssr: {
                output: {
                    sourceMap: {
                        js: isDev ? "inline-source-map" : "source-map",
                    },
                },
                source: {
                    define: {
                        "import.meta.env.SSR": "true",
                        IS_CLOUDFLARE: JSON.stringify(!isDev),
                    },
                },
                tools: {
                    rspack: {
                        optimization: {
                            minimize: false,
                        },
                        plugins: [
                            new optimize.LimitChunkCountPlugin({
                                maxChunks: 1,
                            }),
                        ],
                        module: {
                            parser: {
                                javascript: {
                                    // https://github.com/web-infra-dev/rspack/issues/13046#issuecomment-4131952161
                                    // buggy, doesn't return a URI
                                    importMetaResolve: true,
                                },
                            },
                        },
                        output: {
                            // https://github.com/web-infra-dev/rsbuild/issues/7533
                            devtoolModuleFilenameTemplate: isDev ? "file://[absolute-resource-path]" : undefined,
                        },
                    },
                },
            },
        },
    } satisfies RsbuildConfig;
});
