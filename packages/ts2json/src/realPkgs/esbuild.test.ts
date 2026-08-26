import { expect, it } from "vitest";
import { Analyzer } from "..";
import { __String } from "typescript";
import { BuildOptions as _BuildOptions } from "esbuild";

it("esbuild.d.ts", () => { 
    const analyzer = Analyzer.createFromModule("esbuild");
    const schema = analyzer.getSymbolForExportName("BuildOptions" as __String);
    if (!schema) throw new Error("BuildOptions not found");
    const jsonSchema = analyzer.getSchemaForSymbol(schema);
    expect(jsonSchema).toMatchInlineSnapshot(`
      {
        "additionalProperties": false,
        "properties": {
          "absPaths": {
            "description": "Documentation: https://esbuild.github.io/api/#abs-paths",
            "items": {
              "anyOf": [
                {
                  "const": "code",
                  "type": "string",
                },
                {
                  "const": "log",
                  "type": "string",
                },
                {
                  "const": "metafile",
                  "type": "string",
                },
              ],
            },
            "type": "array",
          },
          "absWorkingDir": {
            "description": "Documentation: https://esbuild.github.io/api/#working-directory",
            "type": "string",
          },
          "alias": {
            "additionalProperties": {
              "type": "string",
            },
            "description": "Documentation: https://esbuild.github.io/api/#alias",
            "properties": {},
            "type": "object",
          },
          "allowOverwrite": {
            "description": "Documentation: https://esbuild.github.io/api/#allow-overwrite",
            "type": "boolean",
          },
          "assetNames": {
            "description": "Documentation: https://esbuild.github.io/api/#asset-names",
            "type": "string",
          },
          "banner": {
            "additionalProperties": {
              "type": "string",
            },
            "description": "Documentation: https://esbuild.github.io/api/#banner",
            "properties": {},
            "type": "object",
          },
          "bundle": {
            "description": "Documentation: https://esbuild.github.io/api/#bundle",
            "type": "boolean",
          },
          "charset": {
            "anyOf": [
              {
                "const": "ascii",
                "type": "string",
              },
              {
                "const": "utf8",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#charset",
          },
          "chunkNames": {
            "description": "Documentation: https://esbuild.github.io/api/#chunk-names",
            "type": "string",
          },
          "color": {
            "description": "Documentation: https://esbuild.github.io/api/#color",
            "type": "boolean",
          },
          "conditions": {
            "description": "Documentation: https://esbuild.github.io/api/#conditions",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "define": {
            "additionalProperties": {
              "type": "string",
            },
            "description": "Documentation: https://esbuild.github.io/api/#define",
            "properties": {},
            "type": "object",
          },
          "drop": {
            "description": "Documentation: https://esbuild.github.io/api/#drop",
            "items": {
              "anyOf": [
                {
                  "const": "console",
                  "type": "string",
                },
                {
                  "const": "debugger",
                  "type": "string",
                },
              ],
            },
            "type": "array",
          },
          "dropLabels": {
            "description": "Documentation: https://esbuild.github.io/api/#drop-labels",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "entryNames": {
            "description": "Documentation: https://esbuild.github.io/api/#entry-names",
            "type": "string",
          },
          "entryPoints": {
            "anyOf": [
              {
                "additionalProperties": {
                  "type": "string",
                },
                "properties": {},
                "type": "object",
              },
              {
                "items": {
                  "anyOf": [
                    {
                      "type": "string",
                    },
                    {
                      "additionalProperties": false,
                      "properties": {
                        "in": {
                          "type": "string",
                        },
                        "out": {
                          "type": "string",
                        },
                      },
                      "required": [
                        "in",
                        "out",
                      ],
                      "type": "object",
                    },
                  ],
                },
                "type": "array",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#entry-points",
          },
          "external": {
            "description": "Documentation: https://esbuild.github.io/api/#external",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "footer": {
            "additionalProperties": {
              "type": "string",
            },
            "description": "Documentation: https://esbuild.github.io/api/#footer",
            "properties": {},
            "type": "object",
          },
          "format": {
            "anyOf": [
              {
                "const": "iife",
                "type": "string",
              },
              {
                "const": "cjs",
                "type": "string",
              },
              {
                "const": "esm",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#format",
          },
          "globalName": {
            "description": "Documentation: https://esbuild.github.io/api/#global-name",
            "type": "string",
          },
          "ignoreAnnotations": {
            "description": "Documentation: https://esbuild.github.io/api/#ignore-annotations",
            "type": "boolean",
          },
          "inject": {
            "description": "Documentation: https://esbuild.github.io/api/#inject",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "jsx": {
            "anyOf": [
              {
                "const": "transform",
                "type": "string",
              },
              {
                "const": "preserve",
                "type": "string",
              },
              {
                "const": "automatic",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#jsx",
          },
          "jsxDev": {
            "description": "Documentation: https://esbuild.github.io/api/#jsx-development",
            "type": "boolean",
          },
          "jsxFactory": {
            "description": "Documentation: https://esbuild.github.io/api/#jsx-factory",
            "type": "string",
          },
          "jsxFragment": {
            "description": "Documentation: https://esbuild.github.io/api/#jsx-fragment",
            "type": "string",
          },
          "jsxImportSource": {
            "description": "Documentation: https://esbuild.github.io/api/#jsx-import-source",
            "type": "string",
          },
          "jsxSideEffects": {
            "description": "Documentation: https://esbuild.github.io/api/#jsx-side-effects",
            "type": "boolean",
          },
          "keepNames": {
            "description": "Documentation: https://esbuild.github.io/api/#keep-names",
            "type": "boolean",
          },
          "legalComments": {
            "anyOf": [
              {
                "const": "external",
                "type": "string",
              },
              {
                "const": "linked",
                "type": "string",
              },
              {
                "const": "inline",
                "type": "string",
              },
              {
                "const": "none",
                "type": "string",
              },
              {
                "const": "eof",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#legal-comments",
          },
          "lineLimit": {
            "description": "Documentation: https://esbuild.github.io/api/#line-limit",
            "type": "number",
          },
          "loader": {
            "additionalProperties": {
              "anyOf": [
                {
                  "const": "base64",
                  "type": "string",
                },
                {
                  "const": "binary",
                  "type": "string",
                },
                {
                  "const": "copy",
                  "type": "string",
                },
                {
                  "const": "css",
                  "type": "string",
                },
                {
                  "const": "dataurl",
                  "type": "string",
                },
                {
                  "const": "default",
                  "type": "string",
                },
                {
                  "const": "empty",
                  "type": "string",
                },
                {
                  "const": "file",
                  "type": "string",
                },
                {
                  "const": "js",
                  "type": "string",
                },
                {
                  "const": "json",
                  "type": "string",
                },
                {
                  "const": "jsx",
                  "type": "string",
                },
                {
                  "const": "local-css",
                  "type": "string",
                },
                {
                  "const": "text",
                  "type": "string",
                },
                {
                  "const": "ts",
                  "type": "string",
                },
                {
                  "const": "tsx",
                  "type": "string",
                },
              ],
            },
            "description": "Documentation: https://esbuild.github.io/api/#loader",
            "properties": {},
            "type": "object",
          },
          "logLevel": {
            "anyOf": [
              {
                "const": "verbose",
                "type": "string",
              },
              {
                "const": "debug",
                "type": "string",
              },
              {
                "const": "info",
                "type": "string",
              },
              {
                "const": "warning",
                "type": "string",
              },
              {
                "const": "error",
                "type": "string",
              },
              {
                "const": "silent",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#log-level",
          },
          "logLimit": {
            "description": "Documentation: https://esbuild.github.io/api/#log-limit",
            "type": "number",
          },
          "logOverride": {
            "additionalProperties": {
              "anyOf": [
                {
                  "const": "verbose",
                  "type": "string",
                },
                {
                  "const": "debug",
                  "type": "string",
                },
                {
                  "const": "info",
                  "type": "string",
                },
                {
                  "const": "warning",
                  "type": "string",
                },
                {
                  "const": "error",
                  "type": "string",
                },
                {
                  "const": "silent",
                  "type": "string",
                },
              ],
            },
            "description": "Documentation: https://esbuild.github.io/api/#log-override",
            "properties": {},
            "type": "object",
          },
          "logStyle": {
            "anyOf": [
              {
                "const": "default",
                "type": "string",
              },
              {
                "const": "visualstudio",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#log-style",
          },
          "mainFields": {
            "description": "Documentation: https://esbuild.github.io/api/#main-fields",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "mangleCache": {
            "additionalProperties": {
              "anyOf": [
                {
                  "type": "string",
                },
                {
                  "const": false,
                  "type": "boolean",
                },
              ],
            },
            "description": "Documentation: https://esbuild.github.io/api/#mangle-props",
            "properties": {},
            "type": "object",
          },
          "mangleProps": {
            "description": "Documentation: https://esbuild.github.io/api/#mangle-props",
            "format": "regex",
            "type": "string",
          },
          "mangleQuoted": {
            "description": "Documentation: https://esbuild.github.io/api/#mangle-props",
            "type": "boolean",
          },
          "metafile": {
            "description": "Documentation: https://esbuild.github.io/api/#metafile",
            "type": "boolean",
          },
          "minify": {
            "description": "Documentation: https://esbuild.github.io/api/#minify",
            "type": "boolean",
          },
          "minifyIdentifiers": {
            "description": "Documentation: https://esbuild.github.io/api/#minify",
            "type": "boolean",
          },
          "minifySyntax": {
            "description": "Documentation: https://esbuild.github.io/api/#minify",
            "type": "boolean",
          },
          "minifyWhitespace": {
            "description": "Documentation: https://esbuild.github.io/api/#minify",
            "type": "boolean",
          },
          "nodePaths": {
            "description": "Documentation: https://esbuild.github.io/api/#node-paths",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "outExtension": {
            "additionalProperties": {
              "type": "string",
            },
            "description": "Documentation: https://esbuild.github.io/api/#out-extension",
            "properties": {},
            "type": "object",
          },
          "outbase": {
            "description": "Documentation: https://esbuild.github.io/api/#outbase",
            "type": "string",
          },
          "outdir": {
            "description": "Documentation: https://esbuild.github.io/api/#outdir",
            "type": "string",
          },
          "outfile": {
            "description": "Documentation: https://esbuild.github.io/api/#outfile",
            "type": "string",
          },
          "packages": {
            "anyOf": [
              {
                "const": "bundle",
                "type": "string",
              },
              {
                "const": "external",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#packages",
          },
          "platform": {
            "anyOf": [
              {
                "const": "browser",
                "type": "string",
              },
              {
                "const": "node",
                "type": "string",
              },
              {
                "const": "neutral",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#platform",
          },
          "plugins": {
            "description": "Documentation: https://esbuild.github.io/plugins/",
            "items": {
              "additionalProperties": false,
              "properties": {
                "name": {
                  "type": "string",
                },
                "setup": {
                  "additionalProperties": false,
                  "properties": {},
                  "type": "object",
                },
              },
              "required": [
                "name",
                "setup",
              ],
              "type": "object",
            },
            "type": "array",
          },
          "preserveSymlinks": {
            "description": "Documentation: https://esbuild.github.io/api/#preserve-symlinks",
            "type": "boolean",
          },
          "publicPath": {
            "description": "Documentation: https://esbuild.github.io/api/#public-path",
            "type": "string",
          },
          "pure": {
            "description": "Documentation: https://esbuild.github.io/api/#pure",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "reserveProps": {
            "description": "Documentation: https://esbuild.github.io/api/#mangle-props",
            "format": "regex",
            "type": "string",
          },
          "resolveExtensions": {
            "description": "Documentation: https://esbuild.github.io/api/#resolve-extensions",
            "items": {
              "type": "string",
            },
            "type": "array",
          },
          "sourceRoot": {
            "description": "Documentation: https://esbuild.github.io/api/#source-root",
            "type": "string",
          },
          "sourcemap": {
            "anyOf": [
              {
                "const": false,
                "type": "boolean",
              },
              {
                "const": true,
                "type": "boolean",
              },
              {
                "const": "external",
                "type": "string",
              },
              {
                "const": "linked",
                "type": "string",
              },
              {
                "const": "inline",
                "type": "string",
              },
              {
                "const": "both",
                "type": "string",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#sourcemap",
          },
          "sourcesContent": {
            "description": "Documentation: https://esbuild.github.io/api/#sources-content",
            "type": "boolean",
          },
          "splitting": {
            "description": "Documentation: https://esbuild.github.io/api/#splitting",
            "type": "boolean",
          },
          "stdin": {
            "additionalProperties": false,
            "description": "Documentation: https://esbuild.github.io/api/#stdin",
            "properties": {
              "contents": {
                "anyOf": [
                  {
                    "type": "string",
                  },
                  {
                    "items": {
                      "maximum": 255,
                      "minimum": 0,
                      "type": "number",
                    },
                    "type": "array",
                  },
                ],
              },
              "loader": {
                "anyOf": [
                  {
                    "const": "base64",
                    "type": "string",
                  },
                  {
                    "const": "binary",
                    "type": "string",
                  },
                  {
                    "const": "copy",
                    "type": "string",
                  },
                  {
                    "const": "css",
                    "type": "string",
                  },
                  {
                    "const": "dataurl",
                    "type": "string",
                  },
                  {
                    "const": "default",
                    "type": "string",
                  },
                  {
                    "const": "empty",
                    "type": "string",
                  },
                  {
                    "const": "file",
                    "type": "string",
                  },
                  {
                    "const": "js",
                    "type": "string",
                  },
                  {
                    "const": "json",
                    "type": "string",
                  },
                  {
                    "const": "jsx",
                    "type": "string",
                  },
                  {
                    "const": "local-css",
                    "type": "string",
                  },
                  {
                    "const": "text",
                    "type": "string",
                  },
                  {
                    "const": "ts",
                    "type": "string",
                  },
                  {
                    "const": "tsx",
                    "type": "string",
                  },
                ],
              },
              "resolveDir": {
                "type": "string",
              },
              "sourcefile": {
                "type": "string",
              },
            },
            "required": [
              "contents",
            ],
            "type": "object",
          },
          "supported": {
            "additionalProperties": {
              "type": "boolean",
            },
            "description": "Documentation: https://esbuild.github.io/api/#supported",
            "properties": {},
            "type": "object",
          },
          "target": {
            "anyOf": [
              {
                "type": "string",
              },
              {
                "items": {
                  "type": "string",
                },
                "type": "array",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#target",
          },
          "treeShaking": {
            "description": "Documentation: https://esbuild.github.io/api/#tree-shaking",
            "type": "boolean",
          },
          "tsconfig": {
            "description": "Documentation: https://esbuild.github.io/api/#tsconfig",
            "type": "string",
          },
          "tsconfigRaw": {
            "anyOf": [
              {
                "type": "string",
              },
              {
                "additionalProperties": false,
                "properties": {
                  "compilerOptions": {
                    "additionalProperties": false,
                    "properties": {
                      "alwaysStrict": {
                        "type": "boolean",
                      },
                      "baseUrl": {
                        "type": "string",
                      },
                      "experimentalDecorators": {
                        "type": "boolean",
                      },
                      "importsNotUsedAsValues": {
                        "anyOf": [
                          {
                            "const": "preserve",
                            "type": "string",
                          },
                          {
                            "const": "error",
                            "type": "string",
                          },
                          {
                            "const": "remove",
                            "type": "string",
                          },
                        ],
                      },
                      "jsx": {
                        "anyOf": [
                          {
                            "const": "preserve",
                            "type": "string",
                          },
                          {
                            "const": "react-native",
                            "type": "string",
                          },
                          {
                            "const": "react",
                            "type": "string",
                          },
                          {
                            "const": "react-jsx",
                            "type": "string",
                          },
                          {
                            "const": "react-jsxdev",
                            "type": "string",
                          },
                        ],
                      },
                      "jsxFactory": {
                        "type": "string",
                      },
                      "jsxFragmentFactory": {
                        "type": "string",
                      },
                      "jsxImportSource": {
                        "type": "string",
                      },
                      "paths": {
                        "additionalProperties": {
                          "items": {
                            "type": "string",
                          },
                          "type": "array",
                        },
                        "properties": {},
                        "type": "object",
                      },
                      "preserveValueImports": {
                        "type": "boolean",
                      },
                      "strict": {
                        "type": "boolean",
                      },
                      "target": {
                        "type": "string",
                      },
                      "useDefineForClassFields": {
                        "type": "boolean",
                      },
                      "verbatimModuleSyntax": {
                        "type": "boolean",
                      },
                    },
                    "type": "object",
                  },
                },
                "type": "object",
              },
            ],
            "description": "Documentation: https://esbuild.github.io/api/#tsconfig-raw",
          },
          "write": {
            "description": "Documentation: https://esbuild.github.io/api/#write",
            "type": "boolean",
          },
        },
        "type": "object",
      }
    `);
})