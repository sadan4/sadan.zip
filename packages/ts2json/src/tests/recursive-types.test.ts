import { dedent } from "../utils";
import { handleDefaultExport } from "..";

import { describe, expect, it } from "vitest";
import { z } from "zod";

describe("ts2json", () => {
    describe("recursive types", () => {
        it("handles a self-referential optional property", () => {
            const input = dedent/*ts*/`
                export default interface Tree {
                    name: string;
                    child?: Tree;
                }
            `;

            const output = handleDefaultExport(input);

            expect(output).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "Tree": {
                    "additionalProperties": false,
                    "properties": {
                      "child": {
                        "$ref": "#/$defs/Tree",
                      },
                      "name": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "name",
                    ],
                    "type": "object",
                  },
                },
                "$ref": "#/$defs/Tree",
                "$schema": "https://json-schema.org/draft/2020-12/schema",
              }
            `);

            const zs = z.fromJSONSchema(output as any);

            expect(zs.parse({
                name: "a",
                child: { name: "b" },
            })).toEqual({
                name: "a",
                child: { name: "b" },
            });
            expect(() => zs.parse({
                name: "a",
                child: { name: 1 },
            })).toThrow(z.ZodError);
        });
        it("handles a self-referential array property", () => {
            const input = dedent/*ts*/`
                export default interface Tree {
                    name: string;
                    children: Tree[];
                }
            `;

            const output = handleDefaultExport(input);

            expect(output).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "Tree": {
                    "additionalProperties": false,
                    "properties": {
                      "children": {
                        "items": {
                          "$ref": "#/$defs/Tree",
                        },
                        "type": "array",
                      },
                      "name": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "name",
                      "children",
                    ],
                    "type": "object",
                  },
                },
                "$ref": "#/$defs/Tree",
                "$schema": "https://json-schema.org/draft/2020-12/schema",
              }
            `);

            const zs = z.fromJSONSchema(output as any);

            const value = {
                name: "a",
                children: [
                    {
                        name: "b",
                        children: [],
                    },
                ],
            };

            expect(zs.parse(value)).toEqual(value);
            expect(() => zs.parse({
                name: "a",
                children: [{ name: "b" }],
            })).toThrow(z.ZodError);
        });
        it("handles mutually recursive interfaces", () => {
            const input = dedent/*ts*/`
                interface B {
                    a: A;
                }
                interface A {
                    b?: B;
                }
                export default interface Foo {
                    a: A;
                }
            `;

            const output = handleDefaultExport(input);

            expect(output).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "A": {
                    "additionalProperties": false,
                    "properties": {
                      "b": {
                        "$ref": "#/$defs/B",
                      },
                    },
                    "type": "object",
                  },
                  "B": {
                    "additionalProperties": false,
                    "properties": {
                      "a": {
                        "$ref": "#/$defs/A",
                      },
                    },
                    "required": [
                      "a",
                    ],
                    "type": "object",
                  },
                },
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "a": {
                    "$ref": "#/$defs/A",
                  },
                },
                "required": [
                  "a",
                ],
                "type": "object",
              }
            `);

            const zs = z.fromJSONSchema(output as any);
            const value = { a: { b: { a: {} } } };

            expect(zs.parse(value)).toEqual(value);
            expect(() => zs.parse({ a: { b: {} } })).toThrow(z.ZodError);
        });
    });
});
