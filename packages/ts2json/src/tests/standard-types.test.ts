import { describe, expect, it } from "vitest";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

describe("standard types", () => { 
    it("handles regexp value types", () => {
        const input = dedent/*ts*/`
            export default interface Foo {
                bar: RegExp;
            }
        `;
        expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "format": "regex",
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
    it("handles an optional regexp", () => {
        const input = dedent/*ts*/`
            export default interface Foo {
                bar?: RegExp;
            }
        `;
        expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "format": "regex",
                "type": "string",
              },
            },
            "type": "object",
          }
        `);
    });
    it("handles an array of regexps", () => {
        const input = dedent/*ts*/`
            export default interface Foo {
                bar: RegExp[];
            }
        `;
        expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "items": {
                  "format": "regex",
                  "type": "string",
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
    it("does not treat a user declared RegExp as the builtin", () => {
        const input = dedent/*ts*/`
            interface RegExp {
                mine: string;
            }
            export default interface Foo {
                bar: RegExp;
            }
        `;
        expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "additionalProperties": false,
                "properties": {
                  "mine": {
                    "type": "string",
                  },
                },
                "required": [
                  "mine",
                ],
                "type": "object",
              },
            },
            "required": [
              "bar",
            ],
            "type": "object",
          }
        `);
    });
    it("handles Uint8Array value types", () => {
        const input = dedent/*ts*/`
            export default interface Foo {
                bar: Uint8Array;
            }
        `;
        expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "items": {
                  "maximum": 255,
                  "minimum": 0,
                  "type": "number",
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
    it("does not treat a user declared Uint8Array as the builtin", () => {
        const input = dedent/*ts*/`
            interface Uint8Array {
                mine: string;
            }
            export default interface Foo {
                bar: Uint8Array;
            }
        `;
        expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
          {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "additionalProperties": false,
            "properties": {
              "bar": {
                "additionalProperties": false,
                "properties": {
                  "mine": {
                    "type": "string",
                  },
                },
                "required": [
                  "mine",
                ],
                "type": "object",
              },
            },
            "required": [
              "bar",
            ],
            "type": "object",
          }
        `);
    });
})
