import { describe, expect, it } from "vitest";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";
import { z } from "zod";

describe("ts2json", () => {
    it("should work", () => {
        const input = dedent/*ts*/`
            export default interface Foo {
                bar: string;
                baz: number;
            }
        `;
        const output = handleDefaultExport(input);
        expect(output).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "type": "string",
              },
              "baz": {
                "type": "number",
              },
            },
            "required": [
              "bar",
              "baz",
            ],
            "type": "object",
          }
        `);
        const zs = z.fromJSONSchema(output as any);
        expect(zs.parse({ bar: "hello", baz: 42 })).toEqual({ bar: "hello", baz: 42 });
        expect(() => zs.parse({ bar: "hello", baz: "not a number" })).toThrow(z.ZodError);
        expect(() => zs.parse({ bar: 123 })).toThrow(z.ZodError);
    });
    it("handles narrowed types in interface inheritance", () => { 
        const input = dedent/*ts*/`
            interface Base {
                foo: string | number;
            }
            export default interface Derived extends Base {
                foo: string;
            }
        `;
        const output = handleDefaultExport(input);
        expect(output).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "foo": {
                "type": "string",
              },
            },
            "required": [
              "foo",
            ],
            "type": "object",
          }
        `);
    });
    it("handles union types", () => { 
        const input = dedent/*ts*/`
            export default interface Foo {
                bar: string | number;
            }
        `;
        const output = handleDefaultExport(input);
        expect(output).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "anyOf": [
                  {
                    "type": "string",
                  },
                  {
                    "type": "number",
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
    })
    it("handles intersection types", () => {
        const input = dedent/*ts*/`
            interface A {
                foo: string;
            }
            interface B {
                bar: number;
            }
            export default interface Foo {
                baz: A & B;
            }
        `;
        const output = handleDefaultExport(input);
        expect(output).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "baz": {
                "allOf": [
                  {
                    "properties": {
                      "foo": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "foo",
                    ],
                    "type": "object",
                  },
                  {
                    "properties": {
                      "bar": {
                        "type": "number",
                      },
                    },
                    "required": [
                      "bar",
                    ],
                    "type": "object",
                  },
                ],
              },
            },
            "required": [
              "baz",
            ],
            "type": "object",
          }
        `);
        const zs = z.fromJSONSchema(output as any);
        zs.parse({ baz: { foo: "hello", bar: 42 } });
        expect(() => zs.parse({ baz: { foo: 1, bar: 42 } })).toThrow(z.ZodError);
        expect(() => zs.parse({ baz: { foo: "hello", bar: "not a number" } })).toThrow(z.ZodError);
    })
});
