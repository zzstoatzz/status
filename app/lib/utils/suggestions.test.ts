import { describe, expect, it } from "vitest";
import { buildSuggestions } from "./suggestions.ts";

describe("buildSuggestions", () => {
  it("leads with what is already showing, so the first frame never jumps", () => {
    expect(buildSuggestions({ current: "🎞️", recent: ["😊"], popular: ["🔥"] })[0]).toBe("🎞️");
  });

  it("puts your own emoji ahead of the site's", () => {
    expect(buildSuggestions({ recent: ["😊", "🐸"], popular: ["🔥", "⭐"] })).toEqual([
      "😊",
      "🐸",
      "🔥",
      "⭐",
    ]);
  });

  it("never repeats one, however many lists it appears in", () => {
    expect(
      buildSuggestions({ current: "😊", recent: ["😊", "🐸"], popular: ["🐸", "😊"] }),
    ).toEqual(["😊", "🐸"]);
  });

  it("caps the cycle so it comes back around", () => {
    const many = Array.from({ length: 40 }, (_, i) => `e${i}`);
    expect(buildSuggestions({ popular: many })).toHaveLength(12);
  });

  it("keeps custom bufos, which are just prefixed values", () => {
    expect(buildSuggestions({ recent: ["custom:bufo-pray"] })).toEqual(["custom:bufo-pray"]);
  });

  it("drops blanks rather than cycling through nothing", () => {
    expect(buildSuggestions({ current: "  ", recent: ["", "😊"], popular: [] })).toEqual(["😊"]);
  });

  it("is empty when there is nothing to suggest", () => {
    expect(buildSuggestions({})).toEqual([]);
  });
});
