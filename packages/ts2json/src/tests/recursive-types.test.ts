import { describe, expect, it } from "vitest";
import { dedent } from "../utils";
import { handleDefaultExport } from "..";

describe("ts2json", () => {
    describe("recursive types", () => {
        it("handles a self-referential optional property", () => {
            const input = dedent/*ts*/`
                export default interface Tree {
                    name: string;
                    child?: Tree;
                }
            `;
            expect(() => handleDefaultExport(input)).not.toThrow();
        });
        it("handles a self-referential array property", () => {
            const input = dedent/*ts*/`
                export default interface Tree {
                    name: string;
                    children: Tree[];
                }
            `;
            expect(() => handleDefaultExport(input)).not.toThrow();
        });
        it("handles mutually recursive interfaces", () => {
            const input = dedent/*ts*/`
                interface B {
                    a: A;
                }
                interface A {
                    b?: B;
                }
                export default interface Foo {
                    a: A;
                }
            `;
            expect(() => handleDefaultExport(input)).not.toThrow();
        });
    });
});
