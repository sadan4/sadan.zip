import type { SchemaObject } from "../schema";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("number literals", () => {
        it("emits a const for a number literal", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: 1;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.bar).toEqual({
                type: "number",
                const: 1,
            });
        });
        it("emits a const for negative and fractional literals", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    neg: -1;
                    frac: 1.5;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.neg).toEqual({
                type: "number",
                const: -1,
            });
            expect(out.properties.frac).toEqual({
                type: "number",
                const: 1.5,
            });
        });
        it("handles a union of number literals", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: 1 | 2;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.bar).toEqual({
                anyOf: [
                    {
                        type: "number",
                        const: 1,
                    },
                    {
                        type: "number",
                        const: 2,
                    },
                ],
            });
        });
        it("handles an implicitly numbered enum", () => {
            const input = dedent/*ts*/`
                enum E {
                    A,
                    B,
                }
                export default interface Foo {
                    bar: E;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.bar).toEqual({
                anyOf: [
                    {
                        type: "number",
                        const: 0,
                    },
                    {
                        type: "number",
                        const: 1,
                    },
                ],
            });
        });
        it("handles an explicitly valued numeric enum member", () => {
            const input = dedent/*ts*/`
                enum E {
                    A = 5,
                }
                export default interface Foo {
                    bar: E.A;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.bar).toEqual({
                type: "number",
                const: 5,
            });
        });
    });
});
