import { handleDefaultExport } from "../internal";
import { dedent } from "../utils";

import { describe, expect, it } from "vitest";

describe("ts2json", () => {
    describe("root schema", () => {
        it("declares the dialect on the root schema", () => {
            const input = dedent/*ts*/`
                export default interface Foo {
                    bar: string;
                }
            `;

            const out = handleDefaultExport(input);

            expect(out.$schema).toBe("https://json-schema.org/draft/2020-12/schema");
        });
        it("does not declare the dialect on nested schemas", () => {
            const input = dedent/*ts*/`
                interface Nested {
                    baz: string;
                }
                export default interface Foo {
                    bar: Nested;
                }
            `;

            const out = handleDefaultExport(input) as any;

            expect(out.properties.bar.$schema).toBeUndefined();
        });
    });
});
