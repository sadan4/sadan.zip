import { handleDefaultExport } from "../internal";
import type { SchemaObject } from "../schema";
import { dedent } from "../utils";

import { describe, expect, it } from "vitest";

/**
 * what a type with no json-representable members currently collapses to.
 * it matches nothing, so a property emitted like this can never validate
 */
const UNSATISFIABLE = {
    type: "object",
    properties: {},
    additionalProperties: false,
};

describe("ts2json", () => {
    describe("function-typed members", () => {
        it("does not emit a function property as a required empty object", () => {
            const input = dedent/*ts*/`
                export default interface Plugin {
                    name: string;
                    setup: (build: string) => void;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.setup).not.toEqual(UNSATISFIABLE);
            expect(out.required ?? []).not.toContain("setup");
        });
        it("does not emit a method signature as a required empty object", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    name: string;
                    run(): void;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;

            expect(out.properties.run).not.toEqual(UNSATISFIABLE);
            expect(out.required ?? []).not.toContain("run");
        });
        it("does not emit symbol-keyed members as property names", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    when: Date;
                }
            `;

            const out = handleDefaultExport(input) as SchemaObject;
            const { when } = out.properties;

            // eg: `__@toPrimitive@620`, which is not a real key
            expect(Object.keys((when as any).properties ?? {}).filter((k) => k.startsWith("__@"))).toEqual([]);
        });
    });
});
