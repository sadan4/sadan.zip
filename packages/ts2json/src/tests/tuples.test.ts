import { handleDefaultExport } from "../internal";
import { dedent } from "../utils";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("tuples", () => {
        it("handles a fixed length tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, number];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "minItems": 2,
                    "prefixItems": [
                      {
                        "type": "string",
                      },
                      {
                        "type": "number",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a tuple with an optional element", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, number?];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "minItems": 1,
                    "prefixItems": [
                      {
                        "type": "string",
                      },
                      {
                        "type": "number",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a tuple with a rest element", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, ...number[]];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
                      "type": "number",
                    },
                    "minItems": 1,
                    "prefixItems": [
                      {
                        "type": "string",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a readonly tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: readonly [string, number];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "minItems": 2,
                    "prefixItems": [
                      {
                        "type": "string",
                      },
                      {
                        "type": "number",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a labeled tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [first: string, ...rest: number[]];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
                      "description": "rest",
                      "type": "number",
                    },
                    "minItems": 1,
                    "prefixItems": [
                      {
                        "description": "first",
                        "type": "string",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles an empty tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "prefixItems": [],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a tuple with a nullable element", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string | null, number];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "minItems": 2,
                    "prefixItems": [
                      {
                        "anyOf": [
                          {
                            "type": "string",
                          },
                          {
                            "type": "null",
                          },
                        ],
                      },
                      {
                        "type": "number",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a tuple of objects", () => {
            const input = dedent/*ts*/`
                interface Item {
                    id: number;
                }
                export default interface Foo {
                    bar: [Item, Item[]];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "Item": {
                    "additionalProperties": false,
                    "properties": {
                      "id": {
                        "type": "number",
                      },
                    },
                    "required": [
                      "id",
                    ],
                    "type": "object",
                  },
                },
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "minItems": 2,
                    "prefixItems": [
                      {
                        "$ref": "#/$defs/Item",
                      },
                      {
                        "items": {
                          "$ref": "#/$defs/Item",
                        },
                        "type": "array",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a nested tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, [number, boolean]][];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
                      "items": false,
                      "minItems": 2,
                      "prefixItems": [
                        {
                          "type": "string",
                        },
                        {
                          "items": false,
                          "minItems": 2,
                          "prefixItems": [
                            {
                              "type": "number",
                            },
                            {
                              "type": "boolean",
                            },
                          ],
                          "type": "array",
                        },
                      ],
                      "type": "array",
                    },
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("throws on a tuple with elements after its rest element", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, ...number[], boolean];
                }
            `;

            expect(() => handleDefaultExport(input)).toThrowErrorMatchingInlineSnapshot("[Error: tuple type [string, ...number[], boolean] has elements after its rest element]");
        });
        it("uses tuple labels as descriptions", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [first: string, second?: number];
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": false,
                    "minItems": 1,
                    "prefixItems": [
                      {
                        "description": "first",
                        "type": "string",
                      },
                      {
                        "description": "second",
                        "type": "number",
                      },
                    ],
                    "type": "array",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
    });
});
