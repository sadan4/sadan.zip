import { handleDefaultExport } from "../internal";
import type { SchemaObject } from "../schema";
import { dedent } from "../utils";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("prototype-shadowing keys", () => {
        it("emits a property literally named __proto__", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    __proto__: string;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;
            const desc = Object.getOwnPropertyDescriptor(out.properties, "__proto__");

            expect(desc?.value).toEqual({ type: "string" });
        });
        it("does not let __proto__ replace the prototype of properties", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    __proto__: { nested: string };
                    other: number;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            // assigning through `__proto__` silently swaps the prototype instead of adding a key
            expect(Object.keys(out.properties)).toEqual(["__proto__", "other"]);
        });
    });
});
