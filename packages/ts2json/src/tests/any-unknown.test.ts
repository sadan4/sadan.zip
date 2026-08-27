import type { SchemaObject } from "../schema";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

import { describe, expect, it } from "vitest";

function messageOf(fn: () => unknown): string {
    try {
        fn();
        return "";
    } catch (e) {
        return (e as Error).message;
    }
}

describe("ts2json", () => {
    describe("any / unknown / never / undefined", () => {
        it("treats a required unknown as unconstrained and required", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    a: unknown;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.required ?? []).toContain("a");
            // `unknown` accepts anything, so it must not be narrowed to a union with null
            expect(out.properties.a).not.toHaveProperty("anyOf");
        });
        it("treats any as unconstrained", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    a: any;
                }
            `;

            expect(() => handleDefaultExport(input)).not.toThrow();
        });
        it("handles a property explicitly typed undefined", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    a?: undefined;
                }
            `;

            expect(() => handleDefaultExport(input)).not.toThrow();
        });
        it("does not report never as array-like", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    a: never;
                }
            `;

            expect(messageOf(() => handleDefaultExport(input))).not.toMatch(/Array-like/);
        });
        it("does not report any as array-like", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    a: any;
                }
            `;

            expect(messageOf(() => handleDefaultExport(input))).not.toMatch(/Array-like/);
        });
    });
});
