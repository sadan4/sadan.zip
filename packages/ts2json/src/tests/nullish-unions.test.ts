import { dedent } from "../utils";
import { handleDefaultExport } from "..";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("unions with null and undefined", () => {
        it("handles a union with null", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string | null;
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
        it("handles a union with undefined", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string | undefined;
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
                "type": "object",
              }
            `);
        });
        it("handles an optional property", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar?: string;
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
                "type": "object",
              }
            `);
        });
        it("handles a union with both null and undefined", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string | null | undefined;
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
                        "type": "string",
                      },
                      {
                        "type": "null",
                      },
                    ],
                  },
                },
                "type": "object",
              }
            `);
        });
        // a bare null property narrows to never at its declaration, which is unimplemented
        it("handles a bare null type", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: null;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "type": "null",
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
