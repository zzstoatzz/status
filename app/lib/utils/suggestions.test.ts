import { describe, expect, it } from "vitest";
import { buildSuggestions, shuffle } from "./suggestions.ts";

/** deterministic stand-in for Math.random */
const seeded = (seed = 1) => {
  let s = seed;
  return () => {
    s = (s * 1103515245 + 12345) % 2147483648;
    return s / 2147483648;
  };
};

describe("shuffle", () => {
  it("keeps every item, without mutating the input", () => {
    const input = ["a", "b", "c", "d"];
    const out = shuffle(input, seeded());
    expect(out.sort()).toEqual(["a", "b", "c", "d"]);
    expect(input).toEqual(["a", "b", "c", "d"]);
  });

  it("actually reorders", () => {
    const input = Array.from({ length: 20 }, (_, i) => String(i));
    expect(shuffle(input, seeded()).join()).not.toBe(input.join());
  });
});

describe("buildSuggestions", () => {
  const recent = ["r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10"];
  const popular = ["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8"];

  it("leads with what is already showing, so the first frame never jumps", () => {
    const out = buildSuggestions({ current: "🎞️", recent, popular, random: seeded() });
    expect(out[0]).toBe("🎞️");
  });

  /** the bug: it walked your history in order, so every pass was identical */
  it("does not replay your history in order", () => {
    const out = buildSuggestions({ recent, popular, random: seeded() });
    expect(out.join()).not.toBe(recent.slice(0, 6).join() + "," + popular.slice(0, 6).join());
    expect(out.slice(0, 5)).not.toEqual(recent.slice(0, 5));
  });

  it("gives a different order on a different draw", () => {
    const a = buildSuggestions({ recent, popular, random: seeded(1) });
    const b = buildSuggestions({ recent, popular, random: seeded(99) });
    expect(a.join()).not.toBe(b.join());
  });

  /** a long history used to fill the cap and shut popular out entirely */
  it("always represents both sources", () => {
    const out = buildSuggestions({ recent, popular, random: seeded() });
    expect(out.some((e) => e.startsWith("r"))).toBe(true);
    expect(out.some((e) => e.startsWith("p"))).toBe(true);
  });

  it("never repeats one, however many lists it appears in", () => {
    const out = buildSuggestions({
      current: "😊",
      recent: ["😊", "🐸"],
      popular: ["🐸", "😊"],
      random: seeded(),
    });
    expect(out).toEqual([...new Set(out)]);
    expect(out.filter((e) => e === "😊")).toHaveLength(1);
  });

  it("caps the cycle so it comes back around", () => {
    const many = Array.from({ length: 40 }, (_, i) => `e${i}`);
    expect(buildSuggestions({ popular: many, random: seeded() }).length).toBeLessThanOrEqual(12);
  });

  it("keeps custom bufos, which are just prefixed values", () => {
    expect(buildSuggestions({ recent: ["custom:bufo-pray"], random: seeded() })).toEqual([
      "custom:bufo-pray",
    ]);
  });

  it("drops blanks rather than cycling through nothing", () => {
    expect(buildSuggestions({ current: "  ", recent: ["", "😊"], random: seeded() })).toEqual([
      "😊",
    ]);
  });

  it("is empty when there is nothing to suggest", () => {
    expect(buildSuggestions({ random: seeded() })).toEqual([]);
  });

  it("works when only one source has anything", () => {
    const out = buildSuggestions({ popular, random: seeded() });
    expect(out.length).toBeGreaterThan(0);
    expect(out.every((e) => popular.includes(e))).toBe(true);
  });
});
