import { describe, expect, it } from "vitest";
import { formatStars, languageColor } from "./format";

describe("formatStars", () => {
  it("returns small counts verbatim", () => {
    expect(formatStars(0)).toBe("0");
    expect(formatStars(42)).toBe("42");
    expect(formatStars(999)).toBe("999");
  });
  it("compacts thousands", () => {
    expect(formatStars(1000)).toBe("1k");
    expect(formatStars(1234)).toBe("1.2k");
    expect(formatStars(1500)).toBe("1.5k");
    expect(formatStars(12345)).toBe("12.3k");
  });
  it("compacts millions", () => {
    expect(formatStars(1_000_000)).toBe("1M");
    expect(formatStars(1_500_000)).toBe("1.5M");
  });
  it("handles invalid input", () => {
    expect(formatStars(-5)).toBe("0");
    expect(formatStars(NaN)).toBe("0");
  });
});

describe("languageColor", () => {
  it("maps known languages", () => {
    expect(languageColor("TypeScript")).toBe("#3178c6");
  });
  it("falls back for unknown / null", () => {
    expect(languageColor("Brainfuck")).toBe("#8793a8");
    expect(languageColor(null)).toBe("#8793a8");
  });
});
