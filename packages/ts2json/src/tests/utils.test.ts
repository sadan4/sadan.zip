import { describe, expect, it } from "vitest";
import { popcnt } from "../utils";

it("popcnt", () => { 
    expect(popcnt(0)).toBe(0);
    expect(popcnt(1)).toBe(1);
    expect(popcnt(2)).toBe(1);
    expect(popcnt(0b11101)).toBe(4);
    expect(popcnt(0b11111111111111111111111111111111)).toBe(32);
    expect(popcnt(0b10101010101010101010101010101010)).toBe(16);
    expect(popcnt(0b10000000000000000000000000000000)).toBe(1);
    expect(popcnt(0b01111111111111111111111111111111)).toBe(31);
})