import { describe, expect, it } from "vitest";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

describe("ts2json", () => {
    describe("boolean literals", () => {
        it("emits a const for true and false", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    t: true;
                    f: false;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "f": {
                    "const": false,
                    "type": "boolean",
                  },
                  "t": {
                    "const": true,
                    "type": "boolean",
                  },
                },
                "required": [
                  "t",
                  "f",
                ],
                "type": "object",
              }
            `);
        });
        it("does not emit a const for a plain boolean", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: boolean;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "type": "boolean",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("collapses a true | false union into a plain boolean", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: true | false;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "type": "boolean",
                  },
                },
                "required": [
                  "bar",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a boolean literal in a union with another type", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: true | string;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "anyOf": [
                      {
                        "type": "string",
                      },
                      {
                        "const": true,
                        "type": "boolean",
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
        it("handles a nullable boolean literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: false | null;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "anyOf": [
                      {
                        "const": false,
                        "type": "boolean",
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
        it("handles an optional boolean literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar?: true;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "const": true,
                    "type": "boolean",
                  },
                },
                "type": "object",
              }
            `);
        });
    });
});
