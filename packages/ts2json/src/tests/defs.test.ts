import { handleDefaultExport } from "../internal";
import { dedent } from "../utils";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("$defs", () => {
        it("inlines a type that is only used once", () => {
            const input = dedent/*ts*/`
                interface Item {
                    id: number;
                }
                export default interface Foo {
                    bar: Item;
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
                      "id": {
                        "type": "number",
                      },
                    },
                    "required": [
                      "id",
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
        it("hoists a type that is used more than once", () => {
            const input = dedent/*ts*/`
                interface Item {
                    id: number;
                }
                interface Only {
                    x: string;
                }
                export default interface Foo {
                    a: Item;
                    b: Item;
                    c: Only;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "Item": {
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
                },
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "a": {
                    "$ref": "#/$defs/Item",
                  },
                  "b": {
                    "$ref": "#/$defs/Item",
                  },
                  "c": {
                    "additionalProperties": false,
                    "properties": {
                      "x": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "x",
                    ],
                    "type": "object",
                  },
                },
                "required": [
                  "a",
                  "b",
                  "c",
                ],
                "type": "object",
              }
            `);
        });
        it("keeps the jsdoc of each reference site", () => {
            const input = dedent/*ts*/`
                interface Item {
                    id: number;
                }
                export default interface Foo {
                    /**
                     * the first one
                     */
                    a: Item;
                    /**
                     * the second one
                     */
                    b: Item;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "Item": {
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
                },
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "a": {
                    "$ref": "#/$defs/Item",
                    "description": "the first one",
                  },
                  "b": {
                    "$ref": "#/$defs/Item",
                    "description": "the second one",
                  },
                },
                "required": [
                  "a",
                  "b",
                ],
                "type": "object",
              }
            `);
        });
        it("suffixes a name claimed by another type", () => {
            const input = dedent/*ts*/`
                namespace First {
                    export interface Options {
                        a: string;
                    }
                }
                namespace Second {
                    export interface Options {
                        b: string;
                    }
                }
                export default interface Foo {
                    one: First.Options;
                    two: First.Options;
                    three: Second.Options;
                    four: Second.Options;
                }
            `;

            expect(handleDefaultExport(input)).toMatchInlineSnapshot(`
              {
                "$defs": {
                  "Options": {
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
                  "Options_2": {
                    "additionalProperties": false,
                    "properties": {
                      "b": {
                        "type": "string",
                      },
                    },
                    "required": [
                      "b",
                    ],
                    "type": "object",
                  },
                },
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "additionalProperties": false,
                "properties": {
                  "four": {
                    "$ref": "#/$defs/Options_2",
                  },
                  "one": {
                    "$ref": "#/$defs/Options",
                  },
                  "three": {
                    "$ref": "#/$defs/Options_2",
                  },
                  "two": {
                    "$ref": "#/$defs/Options",
                  },
                },
                "required": [
                  "one",
                  "two",
                  "three",
                  "four",
                ],
                "type": "object",
              }
            `);
        });
    });
});
