import { describe, expect, it } from "vitest";
import { dedent } from "./utils";
import { handleDefaultExport } from ".";
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
        const parsed = zs.parse({ bar: "hello", baz: 42 });
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
    describe("unions with null and undefined", () => {
        it("handles a union with null", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string | null;
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
    describe("string literals", () => {
        it("emits a const for a string literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: "baz";
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
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
    describe("tuples", () => {
        it.todo("handles a fixed length tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, number];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles a tuple with an optional element", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, number?];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles a tuple with a rest element", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [string, ...number[]];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles a readonly tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: readonly [string, number];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles a labeled tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [first: string, ...rest: number[]];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles an empty tuple", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: [];
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
    });
    describe("other object spellings", () => {
        it("handles an inline object type", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { baz: string };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": false,
                    "properties": {
                      "baz": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "baz",
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
        it("handles a type alias for an object", () => {
            const input = dedent/*ts*/`
                type Bar = {
                    baz: string;
                };
                export default interface Foo {
                    bar: Bar;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": false,
                    "properties": {
                      "baz": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "baz",
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
        it("handles Record with literal keys", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: Record<"a" | "b", number>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": false,
                    "properties": {
                      "a": {
                        "type": "number",
                      },
                      "b": {
                        "type": "number",
                      },
                    },
                    "required": [
                      "a",
                      "b",
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
        it("handles a mapped type over literal keys", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [K in "a" | "b"]: number };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": false,
                    "properties": {
                      "a": {
                        "type": "number",
                      },
                      "b": {
                        "type": "number",
                      },
                    },
                    "required": [
                      "a",
                      "b",
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
        it("handles Partial", () => {
            const input = dedent/*ts*/`
                interface Bar {
                    baz: string;
                }
                export default interface Foo {
                    bar: Partial<Bar>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": false,
                    "properties": {
                      "baz": {
                        "type": "string",
                      },
                    },
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
        it("handles Pick and Omit", () => {
            const input = dedent/*ts*/`
                interface Bar {
                    a: string;
                    b: number;
                }
                export default interface Foo {
                    picked: Pick<Bar, "a">;
                    omitted: Omit<Bar, "a">;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "additionalProperties": false,
                "properties": {
                  "omitted": {
                    "additionalProperties": false,
                    "properties": {
                      "b": {
                        "type": "number",
                      },
                    },
                    "required": [
                      "b",
                    ],
                    "type": "object",
                  },
                  "picked": {
                    "additionalProperties": false,
                    "properties": {
                      "a": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "a",
                    ],
                    "type": "object",
                  },
                },
                "required": [
                  "picked",
                  "omitted",
                ],
                "type": "object",
              }
            `);
        });
        it.todo("handles an interface with an index signature alongside known properties", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    known: number;
                    [key: string]: number;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles a string index signature", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [key: string]: number };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles a number index signature", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [key: number]: string };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
        it.todo("handles Record with a string key", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: Record<string, number>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot();
        });
    });
})