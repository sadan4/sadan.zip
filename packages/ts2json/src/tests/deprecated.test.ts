import { handleDefaultExport } from "../internal";
import { dedent } from "../utils";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("deprecated symbols", () => {
        it("marks a property deprecated when the tag has a reason", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /**
                     * the bar
                     * @deprecated use baz
                     */
                    bar: string;
                    baz: string;
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
                  "baz": {
                    "type": "string",
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
        it("marks a property deprecated when the tag has no reason", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /** @deprecated */
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
        it("inherits the deprecated tag from the base interface", () => {
            const input = dedent/*ts*/`
                interface Base {
                    /**
                     * @deprecated gone soon
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
                    "deprecated": true,
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
        it("marks deprecated properties of nested objects", () => {
            const input = dedent/*ts*/`
                interface Nested {
                    /** @deprecated inner is gone */
                    inner: string;
                }
                export default interface Foo {
                    /** @deprecated outer is gone */
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
                    "deprecated": true,
                    "properties": {
                      "inner": {
                        "deprecated": true,
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
        it("does not mark undeprecated properties", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    /**
                     * not deprecated, just documented
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
                    "description": "not deprecated, just documented",
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
    });
});
