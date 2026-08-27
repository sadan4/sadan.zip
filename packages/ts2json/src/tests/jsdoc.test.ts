import { dedent } from "../utils";
import { handleDefaultExport } from "..";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("jsdoc descriptions", () => {
        it("adds a description from a jsdoc comment", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /**
                     * the bar
                     */
                    bar: string;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "description": "the bar",
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
        it("joins multi line descriptions with newlines", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /**
                     * first line
                     *
                     * third line
                     */
                    bar: string;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "description": "first line

              third line",
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
        it("excludes jsdoc tags from the description", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /**
                     * the bar
                     * @deprecated use baz
                     * @see something
                     */
                    bar: string;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "deprecated": true,
                    "description": "the bar",
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
        it("omits the description when there is no jsdoc", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    // not a jsdoc comment
                    bar: string;
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
        it("omits the description when the jsdoc has only tags", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /**
                     * @deprecated use baz
                     */
                    bar: string;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "deprecated": true,
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
        it("describes nested object properties", () => {
            const input = dedent/*ts*/`
                interface Nested {
                    /**
                     * the inner one
                     */
                    inner: string;
                }
                export default interface Foo {
                    /**
                     * the outer one
                     */
                    outer: Nested;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "outer": {
                    "additionalProperties": false,
                    "description": "the outer one",
                    "properties": {
                      "inner": {
                        "description": "the inner one",
                        "type": "string",
                      },
                    },
                    "required": [
                      "inner",
                    ],
                    "type": "object",
                  },
                },
                "required": [
                  "outer",
                ],
                "type": "object",
              }
            `);
        });
        it("inherits the description from the base interface", () => {
            const input = dedent/*ts*/`
                interface Base {
                    /**
                     * documented on the base
                     */
                    foo: string | number;
                }
                export default interface Derived extends Base {
                    foo: string;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "foo": {
                    "description": "documented on the base",
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
    });
});
