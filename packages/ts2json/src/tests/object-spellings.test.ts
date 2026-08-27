import { describe, expect, it } from "vitest";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

describe("ts2json", () => {
    describe("other object spellings", () => {
        it("handles an inline object type", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { baz: string };
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
                "$schema": "https://json-schema.org/draft/2020-12/schema",
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
                "$schema": "https://json-schema.org/draft/2020-12/schema",
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
        it("handles Record with literal keys and an undefined value", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: Record<"a" | "b", string | undefined>;
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
                      "a": {
                        "type": "string",
                      },
                      "b": {
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
        it("handles a mapped type over literal keys", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [K in "a" | "b"]: number };
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
                "$schema": "https://json-schema.org/draft/2020-12/schema",
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
                "$schema": "https://json-schema.org/draft/2020-12/schema",
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
        it("handles an interface with an index signature alongside known properties", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    known: number;
                    [key: string]: number;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": {
                  "type": "number",
                },
                "properties": {
                  "known": {
                    "type": "number",
                  },
                },
                "required": [
                  "known",
                ],
                "type": "object",
              }
            `);
        });
        it("handles a string index signature", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [key: string]: number };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": {
                      "type": "number",
                    },
                    "properties": {},
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
        it("handles a number index signature", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [key: number]: string };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": false,
                    "patternProperties": {
                      "^-?\\d+$": {
                        "type": "string",
                      },
                    },
                    "properties": {},
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
        it("handles Record with a string key", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: Record<string, number>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": {
                      "type": "number",
                    },
                    "properties": {},
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
        it("handles both index signatures at once", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [key: string]: string | number, [key: number]: number };
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": {
                      "anyOf": [
                        {
                          "type": "string",
                        },
                        {
                          "type": "number",
                        },
                      ],
                    },
                    "patternProperties": {
                      "^-?\\d+$": {
                        "type": "number",
                      },
                    },
                    "properties": {},
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
        it("handles an index signature of objects", () => {
            const input = dedent/*ts*/`
                interface Item {
                    id: number;
                }
                export default interface Foo {
                    bar: Record<string, Item>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": {
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
                    "properties": {},
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
        it("handles an index signature with a nullable value", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: Record<string, string | null>;
                }
            `;
            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "bar": {
                    "additionalProperties": {
                      "anyOf": [
                        {
                          "type": "string",
                        },
                        {
                          "type": "null",
                        },
                      ],
                    },
                    "properties": {},
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
        it("throws on an index signature with an unsupported key type", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: { [key: symbol]: string };
                }
            `;
            expect(() => handleDefaultExport(input)).toThrowErrorMatchingInlineSnapshot(`[Error: index signature on { [key: symbol]: string; } has an unsupported key type symbol]`);
        });
    });
});
