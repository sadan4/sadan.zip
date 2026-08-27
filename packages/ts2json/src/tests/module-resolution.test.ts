import { Analyzer } from "..";

import { join } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * the module is resolved relative to this file, which does not need to exist
 */
const CONTAINING_FILE = join(import.meta.dirname, "fixtures", "no-types", "entry.ts");

describe("ts2json", () => {
    describe("createFromModule", () => {
        it("rejects a module that only resolves to .mjs", () => {
            expect(() => Analyzer.createFromModule("mjs-only-pkg", CONTAINING_FILE))
                .toThrow(/no type declarations/);
        });
        it("rejects a module that only resolves to .cjs", () => {
            expect(() => Analyzer.createFromModule("cjs-only-pkg", CONTAINING_FILE))
                .toThrow(/no type declarations/);
        });
    });
});
