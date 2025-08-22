// src/test/sanity/basic.test.ts
import { describe, expect, it } from "vitest";

describe("Sanity check", () => {
    it("true should be true", () => {
        expect(true)
            .toBe(true);
    });

    it("should do basic math", () => {
        expect(1 + 1)
            .toBe(2);
        expect(2 * 2)
            .toBe(4);
    });
});
