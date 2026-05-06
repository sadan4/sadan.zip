import { defineConfig, type RsbuildConfig } from "@rsbuild/core";
import { pluginBabel } from "@rsbuild/plugin-babel";
import { pluginNodePolyfill } from "@rsbuild/plugin-node-polyfill";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSass } from "@rsbuild/plugin-sass";
import { RsdoctorRspackPlugin } from "@rsdoctor/rspack-plugin";
import { experiments, type ExternalItemFunctionData, type ExternalItemValue, optimize } from "@rspack/core";
import tailwindPostCss from "@tailwindcss/postcss";
import { tanstackStart } from "@tanstack/react-start/plugin/rsbuild";

import { builtinModules } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const polyfilledSsrModules = [
    "fs",
    "inspector",
    "os",
    "perf_hooks",
    "module",
]
    .flatMap((mod) => [mod, `node:${mod}`]);


export default defineConfig(({ envMode }) => {
    const isDev = envMode === "development";
    const __dirname = dirname(fileURLToPath(import.meta.url));
    const srcDir = join(__dirname, "src");

    return {
        plugins: [
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
                    pureFunctions: true,
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
                plugins: [process.env.RSDOCTOR && new RsdoctorRspackPlugin()],
                optimization: {
                    concatenateModules: true,
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
                plugins: [pluginNodePolyfill()],
                output: {
                    sourceMap: {
                        js: "source-map",
                    },
                    distPath: {
                        js: "j",
                        jsAsync: "j",
                    },
                },
                tools: {
                    rspack: {
                        optimization: {
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
                plugins: [
                    pluginNodePolyfill({
                        include: polyfilledSsrModules,
                    }),
                ],
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
                            // Insane workaround for cloudflare workers not supporting `import.meta.url`
                            importMetaName: '{url: "file://"}',
                        },
                        target: "node",
                        externals({ request: id }: ExternalItemFunctionData): ExternalItemValue | undefined {
                            if (id?.startsWith("node:") || builtinModules.includes(id!)) {
                                return `module ${id}`;
                            }
                        },
                    },
                },
            },
        },
    } satisfies RsbuildConfig;
});
