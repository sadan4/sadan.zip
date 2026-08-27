import { createAnalyzerFromModule } from "..";

// eslint-disable-next-line unused-imports/no-unused-imports
import type { MinifyOptions as _MinifyOptions } from "rolldown";
import type { __String } from "typescript";
import { expect, it } from "vitest";

it("oxc-minify.d.ts", () => {
    const analyzer = createAnalyzerFromModule("rolldown");
    const schema = analyzer.getSymbolForExportName("MinifyOptions" as __String);

    if (!schema) {
        throw new Error("MinifyOptions not found");
    }

    const out = analyzer.getSchemaForSymbol(schema);

    expect(out).toMatchInlineSnapshot(`
      {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "additionalProperties": false,
        "properties": {
          "codegen": {
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
                "additionalProperties": false,
                "properties": {
                  "legalComments": {
                    "anyOf": [
                      {
                        "const": "none",
                        "type": "string",
                      },
                      {
                        "const": "inline",
                        "type": "string",
                      },
                      {
                        "const": "eof",
                        "type": "string",
                      },
                      {
                        "const": "external",
                        "type": "string",
                      },
                      {
                        "additionalProperties": false,
                        "properties": {
                          "linked": {
                            "type": "string",
                          },
                        },
                        "required": [
                          "linked",
                        ],
                        "type": "object",
                      },
                    ],
                    "description": "How to handle legal comments (comments containing \`@license\`, \`@preserve\`, or starting with \`//!\`/\`/*!\`).

      * \`"none"\` - Do not preserve any legal comments.
      * \`"inline"\` - Preserve all legal comments inline.
      * \`"eof"\` - Move all legal comments to the end of the file.
      * \`"external"\` - Extract legal comments without linking.
      * \`{ linked: "path/to/legal.txt" }\` - Extract legal comments and add a link comment to the given path.",
                  },
                  "removeWhitespace": {
                    "description": "Remove whitespace.",
                    "type": "boolean",
                  },
                },
                "type": "object",
              },
            ],
          },
          "compress": {
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
                "additionalProperties": false,
                "properties": {
                  "dropConsole": {
                    "description": "Pass true to discard calls to \`console.*\`.",
                    "type": "boolean",
                  },
                  "dropDebugger": {
                    "description": "Remove \`debugger;\` statements.",
                    "type": "boolean",
                  },
                  "dropLabels": {
                    "description": "Set of label names to drop from the code.

      Labeled statements matching these names will be removed during minification.",
                    "items": {
                      "type": "string",
                    },
                    "type": "array",
                  },
                  "joinVars": {
                    "description": "Join consecutive var, let and const statements.",
                    "type": "boolean",
                  },
                  "keepNames": {
                    "additionalProperties": false,
                    "description": "Keep function / class names.",
                    "properties": {
                      "class": {
                        "description": "Keep class names so that \`Class.prototype.name\` is preserved.

      This does not guarantee that the \`undefined\` name is preserved.",
                        "type": "boolean",
                      },
                      "function": {
                        "description": "Keep function names so that \`Function.prototype.name\` is preserved.

      This does not guarantee that the \`undefined\` name is preserved.",
                        "type": "boolean",
                      },
                    },
                    "required": [
                      "function",
                      "class",
                    ],
                    "type": "object",
                  },
                  "maxIterations": {
                    "description": "Limit the maximum number of iterations for debugging purpose.",
                    "type": "number",
                  },
                  "sequences": {
                    "description": "Join consecutive simple statements using the comma operator.

      \`a; b\` -> \`a, b\`",
                    "type": "boolean",
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
                    "description": "Set desired EcmaScript standard version for output.

      Set \`esnext\` to enable all target highering.

      Example:

      * \`'es2015'\`
      * \`['es2020', 'chrome58', 'edge16', 'firefox57', 'node12', 'safari11']\`",
                  },
                  "treeshake": {
                    "additionalProperties": false,
                    "description": "Treeshake options.",
                    "properties": {
                      "annotations": {
                        "description": "Whether to respect the pure annotations.

      Pure annotations are comments that mark an expression as pure.
      For example:",
                        "type": "boolean",
                      },
                      "invalidImportSideEffects": {
                        "description": "Whether invalid import statements have side effects.

      Accessing a non-existing import name will throw an error.
      Also import statements that cannot be resolved will throw an error.",
                        "type": "boolean",
                      },
                      "manualPureFunctions": {
                        "description": "Whether to treat this function call as pure.

      This function is called for normal function calls, new calls, and
      tagged template calls.",
                        "items": {
                          "type": "string",
                        },
                        "type": "array",
                      },
                      "propertyReadSideEffects": {
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
                            "const": "always",
                            "type": "string",
                          },
                        ],
                        "description": "Whether property read accesses have side effects.",
                      },
                      "propertyWriteSideEffects": {
                        "description": "Whether property write accesses (assignments to member expressions) have side effects.

      When false, assignments like \`obj.prop = value\` are considered side-effect-free
      (assuming the object and value expressions themselves are side-effect-free).",
                        "type": "boolean",
                      },
                      "unknownGlobalSideEffects": {
                        "description": "Whether accessing a global variable has side effects.

      Accessing a non-existing global variable will throw an error.
      Global variable may be a getter that has side effects.",
                        "type": "boolean",
                      },
                    },
                    "type": "object",
                  },
                  "unused": {
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
                        "const": "keep_assign",
                        "type": "string",
                      },
                    ],
                    "description": "Pass \`true\` to drop unreferenced functions and variables.

      Simple direct variable assignments do not count as references unless set to \`keep_assign\`.",
                  },
                },
                "type": "object",
              },
            ],
          },
          "mangle": {
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
                "additionalProperties": false,
                "properties": {
                  "debug": {
                    "description": "Debug mangled names.",
                    "type": "boolean",
                  },
                  "keepNames": {
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
                        "additionalProperties": false,
                        "properties": {
                          "class": {
                            "description": "Preserve \`name\` property for classes.",
                            "type": "boolean",
                          },
                          "function": {
                            "description": "Preserve \`name\` property for functions.",
                            "type": "boolean",
                          },
                        },
                        "required": [
                          "function",
                          "class",
                        ],
                        "type": "object",
                      },
                    ],
                    "description": "Preserve \`name\` property for functions and classes.",
                  },
                  "reserved": {
                    "description": "Names that bindings must not be renamed to, and that bindings already
      carrying them keep. Equivalent to terser's \`mangle.reserved\`.

      Pass \`['exports', 'module']\` when minifying prebuilt CommonJS / UMD files
      that Node consumers \`import\` directly, so Node's cjs-module-lexer can still
      detect the mangled module's named exports.",
                    "items": {
                      "type": "string",
                    },
                    "type": "array",
                  },
                  "toplevel": {
                    "description": "Pass \`true\` to mangle names declared in the top level scope.",
                    "type": "boolean",
                  },
                },
                "type": "object",
              },
            ],
          },
        },
        "type": "object",
      }
    `);
});

