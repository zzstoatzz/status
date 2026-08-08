import { describe, expect, it, vi } from "vitest";
import { bufoImageUrl, resolveBufoUrl } from "./emoji.ts";

/** loadPopularEmoji memoizes per module instance, so each case gets a fresh one. */
async function freshEmojiModule() {
  vi.resetModules();
  return import("./emoji.ts");
}

describe("bufoImageUrl", () => {
  // regression: bufo-stopsign exists only as a .gif. the old client-side
  // png -> gif candidate walk could never recover for an SSR'd <img>, which
  // 404s before hydration attaches onerror. one resolver url has no such race.
  it("returns a single resolver url that is format-agnostic", () => {
    expect(bufoImageUrl("bufo-stopsign")).toBe("https://find-bufo.com/e/bufo-stopsign.png");
  });

  it("percent-encodes each path segment", () => {
    expect(bufoImageUrl("bufo/says hi")).toBe("https://find-bufo.com/e/bufo/says%20hi.png");
  });

  it("does not double-encode an already-encoded name", () => {
    expect(bufoImageUrl("says%20hi")).toBe("https://find-bufo.com/e/says%20hi.png");
  });
});

describe("resolveBufoUrl", () => {
  it("falls through to the gif when the png 404s", async () => {
    const fetchFn = (async (url: string | URL) =>
      new Response(null, {
        status: String(url).endsWith(".gif") ? 200 : 404,
      })) as unknown as typeof fetch;

    await expect(resolveBufoUrl("bufo-stopsign", fetchFn)).resolves.toBe(
      "https://all-the.bufo.zone/bufo-stopsign.gif",
    );
  });

  it("falls back to the first candidate when nothing responds", async () => {
    const fetchFn = (async () => {
      throw new Error("network down");
    }) as unknown as typeof fetch;

    await expect(resolveBufoUrl("nope", fetchFn)).resolves.toBe(
      "https://all-the.bufo.zone/nope.png",
    );
  });
});

describe("loadPopularEmoji", () => {
  // the ⭐ tab used to render a hardcoded array that no usage ever touched.
  // it is now the all-time global ranking, computed server-side.
  it("returns the feed's emoji in the order the server ranked them", async () => {
    const { loadPopularEmoji } = await freshEmojiModule();
    const fetchFeed = vi.fn(async () => ({
      items: [
        { emoji: "😊", text: "a" },
        { emoji: "🥔", text: "b" },
        { emoji: "custom:bufo-alarma", text: "c" },
      ],
    }));

    await expect(loadPopularEmoji(fetchFeed)).resolves.toEqual(["😊", "🥔", "custom:bufo-alarma"]);
  });

  it("memoizes, so reopening the picker does not refetch", async () => {
    const { loadPopularEmoji } = await freshEmojiModule();
    const fetchFeed = vi.fn(async () => ({ items: [{ emoji: "😊" }] }));

    await loadPopularEmoji(fetchFeed);
    await loadPopularEmoji(fetchFeed);

    expect(fetchFeed).toHaveBeenCalledTimes(1);
  });

  it("falls back to the seed list when the index is empty", async () => {
    const { loadPopularEmoji, DEFAULT_FREQUENT } = await freshEmojiModule();
    const fetchFeed = vi.fn(async () => ({ items: [] }));

    // a cold index must never render an empty first tab
    await expect(loadPopularEmoji(fetchFeed)).resolves.toEqual(DEFAULT_FREQUENT);
  });

  it("drops items the feed returned without an emoji", async () => {
    const { loadPopularEmoji } = await freshEmojiModule();
    const fetchFeed = vi.fn(async () => ({
      items: [{ emoji: "😊" }, { text: "no emoji" }, { emoji: "" }, { emoji: "🥔" }],
    }));

    await expect(loadPopularEmoji(fetchFeed)).resolves.toEqual(["😊", "🥔"]);
  });
});
