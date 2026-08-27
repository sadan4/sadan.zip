import { dedent } from "../utils";
import { handleDefaultExport } from "..";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("string literals", () => {
        it("emits a const for a string literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: "baz";
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "const": "baz",
                    "type": "string",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("keeps quotes and escapes in the literal value", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: "he said \"hi\"\n";
                    empty: "";
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "const": "he said "hi"
              ",
                    "type": "string",
                  },
                  "empty": {
                    "const": "",
                    "type": "string",
                  },
                },
                "required": [
                  "bar",
                  "empty",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a union of string literals", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: "a" | "b" | "c";
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "anyOf": [
                      {
                        "const": "a",
                        "type": "string",
                      },
                      {
                        "const": "b",
                        "type": "string",
                      },
                      {
                        "const": "c",
                        "type": "string",
                      },
                    ],
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a string literal in a union with string", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: "a" | string;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "type": "string",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a nullable string literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: "a" | null;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "anyOf": [
                      {
                        "const": "a",
                        "type": "string",
                      },
                      {
                        "type": "null",
                      },
                    ],
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles an optional string literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar?: "a";
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "const": "a",
                    "type": "string",
                  },
                },
                "type": "object",
              }
            `);
        });
        it("handles string enum members", () => {
            const input = dedent/*ts*/`
                enum E {
                    A = "a",
                    B = "b",
                }
                export default interface Foo {
                    bar: E.A;
                    baz: E;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "const": "a",
                    "type": "string",
                  },
                  "baz": {
                    "anyOf": [
                      {
                        "const": "a",
                        "type": "string",
                      },
                      {
                        "const": "b",
                        "type": "string",
                      },
                    ],
                  },
                },
                "required": [
                  "bar",
                  "baz",
                ],
                "type": "object",
              }
            `);
        });
    });
});
