import { describe, expect, it } from "vitest";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

describe("ts2json", () => {
    describe("arrays", () => {
        it("handles an array of a primitive", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string[];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
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
        it("handles the generic array spelling", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: Array<number>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
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
        it("handles a readonly array", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: readonly string[];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
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
        it("handles a nested array", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string[][];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
                      "items": {
                        "type": "string",
                      },
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
        it("handles an array of objects", () => {
            const input = dedent/*ts*/`
                interface Item {
                    id: number;
                }
                export default interface Foo {
                    bar: Item[];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
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
        it("handles an array of a union", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: (string | number)[];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
                      "anyOf": [
                        {
                          "type": "string",
                        },
                        {
                          "type": "number",
                        },
                      ],
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
        it("handles a nullable array", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string[] | null;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "anyOf": [
                      {
                        "items": {
                          "type": "string",
                        },
                        "type": "array",
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
        it("handles an optional array", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar?: string[];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "items": {
                      "type": "string",
                    },
                    "type": "array",
                  },
                },
                "type": "object",
              }
            `);
        });
    });
});
