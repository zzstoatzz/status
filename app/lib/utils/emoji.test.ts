import { describe, expect, it } from "vitest";
import { bufoImageUrl, resolveBufoUrl } from "./emoji.ts";

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
