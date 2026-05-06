import { defineConfig, type RsbuildConfig } from "@rsbuild/core";
import { tanstackStart } from "@tanstack/react-start/plugin/rsbuild";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginBabel } from "@rsbuild/plugin-babel";
import { pluginNodePolyfill } from "@rsbuild/plugin-node-polyfill";
import { pluginSass } from "@rsbuild/plugin-sass";

import tailwindPostCss from "@tailwindcss/postcss";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export default defineConfig(({envMode}) => {
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
                    enabled: false,
                    failOnError: false,
                }
            }),
            pluginSass({
                sassLoaderOptions: {
                    sassOptions: {
                        loadPaths: [join(srcDir, "styles")],
                    }
                }
            })
        ],
        output: {
            sourceMap: {
                extract: true,
                css: true,
            },
            cssModules: {
                exportLocalsConvention: "camelCaseOnly",
                namedExport: true,
                localIdentName: isDev ? "[local]__[hash:base64:6]" : "[hash:base64:6]",
            }
        },
        source: {
            assetsInclude: [/\.wasm$/],
        },
        tools: {
            postcss: {
                postcssOptions: {
                    plugins: [
                        tailwindPostCss()
                    ],
                },
            },
            rspack: {
                module: {
                    rules: [
                        {
                            resourceQuery: /\?raw$/,
                            type: "asset/source",
                        }
                    ]
                },
                optimization: {
                    splitChunks: {
                        cacheGroups: {
                            "global-css": {
                                test: /src\/index\.css/,
                                chunks: "all",
                                minSize: 0,
                                name: "global-css",
                                priority: 10_000,
                            }
                        }
                    }
                }
            }
        },
        performance: {
            buildCache: true,
        },
        environments: {
            client: {
                source: {
                    define: {
                        "import.meta.env.SSR": "false",
                        "IS_CLOUDFLARE": "false",
                    }
                },
                output: {
                    sourceMap: {
                        js: "source-map",
                    }
                },
            },
            ssr: {
                output: {
                    sourceMap: {
                        js: isDev ? "inline-source-map" : "source-map",
                    }
                },
                source: {
                    define: {
                        "import.meta.env.SSR": "true",
                        "IS_CLOUDFLARE": JSON.stringify(!isDev),
                    }
                },
                tools: {
                    rspack: {
                        module: {
                            parser: {
                                javascript: {
                                    // https://github.com/web-infra-dev/rspack/issues/13046#issuecomment-4131952161
                                    // buggy, doesn't return a URI
                                    importMetaResolve: true,
                                }
                            }
                        },
                        output: {
                            // https://github.com/web-infra-dev/rsbuild/issues/7533
                            devtoolModuleFilenameTemplate: isDev ? "file://[absolute-resource-path]" : undefined,
                        }
                    }
                }
            }
        }
    } satisfies RsbuildConfig;
});