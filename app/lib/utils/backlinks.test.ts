import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  _resetBacklinkCache,
  bskyPaths,
  fetchBacklink,
  newest,
  sourceParam,
  statusPermalink,
} from "./backlinks.ts";
import { postLink } from "../atclients.ts";

const DID = "did:plc:xbtmt2zjwlrfegqvch7fboei";
const PERMALINK = `https://status.zzstoatzz.io/status/${DID}/3msm4rddj7q2j`;

// the real shape constellation returned for that permalink
const LINKS_ALL = {
  links: { "app.bsky.feed.post": { ".embed.external.uri": { records: 1, distinct_dids: 1 } } },
};

const ok = (body: unknown) => ({ ok: true, json: async () => body }) as Response;

describe("statusPermalink", () => {
  it("builds the url the copy button produces, with no trailing slash", () => {
    expect(
      statusPermalink(
        "https://status.zzstoatzz.io",
        `at://${DID}/io.zzstoatzz.status.record/3msm4rddj7q2j`,
      ),
    ).toBe(PERMALINK);
  });
});

describe("bskyPaths", () => {
  it("finds the paths that actually have links", () => {
    expect(bskyPaths(LINKS_ALL)).toEqual([{ path: ".embed.external.uri", records: 1 }]);
  });

  it("drops paths constellation reports with zero records", () => {
    // /links/all lists known paths even at zero; those are not worth a request
    expect(
      bskyPaths({
        links: {
          "app.bsky.feed.post": {
            ".embed.external.uri": { records: 0 },
            ".facets[].features[app.bsky.richtext.facet#link].uri": { records: 3 },
          },
        },
      }),
    ).toEqual([{ path: ".facets[].features[app.bsky.richtext.facet#link].uri", records: 3 }]);
  });

  it("orders by most-linked, so the follow-up asks the richest path", () => {
    expect(
      bskyPaths({
        links: {
          "app.bsky.feed.post": { ".a": { records: 2 }, ".b": { records: 9 } },
        },
      }).map((p) => p.path),
    ).toEqual([".b", ".a"]);
  });

  it("ignores other collections and an empty response", () => {
    expect(bskyPaths({ links: { "sh.tangled.repo": { ".website": { records: 1 } } } })).toEqual([]);
    expect(bskyPaths({ links: {} })).toEqual([]);
    expect(bskyPaths({})).toEqual([]);
  });
});

describe("sourceParam", () => {
  it("drops the leading dot constellation reports but does not accept", () => {
    expect(sourceParam(".embed.external.uri")).toBe("app.bsky.feed.post:embed.external.uri");
  });

  it("leaves a facet path's brackets and hash intact for the caller to encode", () => {
    expect(sourceParam(".facets[].features[app.bsky.richtext.facet#link].uri")).toBe(
      "app.bsky.feed.post:facets[].features[app.bsky.richtext.facet#link].uri",
    );
  });
});

describe("newest", () => {
  // rkeys are TIDs, so lexicographic order is creation order
  it("picks the latest rkey regardless of response order", () => {
    expect(newest([{ rkey: "3aaa" }, { rkey: "3zzz" }, { rkey: "3mmm" }])?.rkey).toBe("3zzz");
  });

  it("is null for nothing usable", () => {
    expect(newest([])).toBeNull();
    expect(newest([{ rkey: "" }])).toBeNull();
  });
});

describe("fetchBacklink", () => {
  beforeEach(() => _resetBacklinkCache());

  it("resolves a permalink to the referencing post", async () => {
    const fetchFn = vi
      .fn()
      .mockResolvedValueOnce(ok(LINKS_ALL))
      .mockResolvedValueOnce(ok({ records: [{ did: DID, rkey: "3msm4v3sdnc2q" }] }));

    expect(await fetchBacklink(PERMALINK, fetchFn as never)).toEqual({
      count: 1,
      did: DID,
      rkey: "3msm4v3sdnc2q",
    });

    // the target must be the permalink exactly — constellation treats a
    // trailing slash as a different target
    expect(fetchFn.mock.calls[0][0]).toContain(encodeURIComponent(PERMALINK));
    expect(fetchFn.mock.calls[1][0]).toContain(
      encodeURIComponent("app.bsky.feed.post:embed.external.uri"),
    );
  });

  it("costs one request for a status nobody linked", async () => {
    const fetchFn = vi.fn().mockResolvedValueOnce(ok({ links: {} }));
    expect(await fetchBacklink(PERMALINK, fetchFn as never)).toBeNull();
    expect(fetchFn).toHaveBeenCalledTimes(1);
  });

  it("sums counts across paths but links to the newest post", async () => {
    const fetchFn = vi
      .fn()
      .mockResolvedValueOnce(
        ok({
          links: {
            "app.bsky.feed.post": { ".embed.external.uri": { records: 2 }, ".x": { records: 1 } },
          },
        }),
      )
      .mockResolvedValueOnce(
        ok({
          records: [
            { did: DID, rkey: "3aaa" },
            { did: "did:plc:other", rkey: "3zzz" },
          ],
        }),
      );

    expect(await fetchBacklink(PERMALINK, fetchFn as never)).toEqual({
      count: 3,
      did: "did:plc:other",
      rkey: "3zzz",
    });
  });

  it("percent-encodes a facet path, so the # is not read as a fragment", async () => {
    const fetchFn = vi
      .fn()
      .mockResolvedValueOnce(
        ok({
          links: {
            "app.bsky.feed.post": {
              ".facets[].features[app.bsky.richtext.facet#link].uri": { records: 1 },
            },
          },
        }),
      )
      .mockResolvedValueOnce(ok({ records: [{ did: DID, rkey: "3aaa" }] }));

    await fetchBacklink(PERMALINK, fetchFn as never);
    const url = fetchFn.mock.calls[1][0] as string;
    expect(url).toContain("%23link");
    expect(url).not.toContain("#link");
  });

  it("caches, so scrolling a feed does not re-ask", async () => {
    const fetchFn = vi.fn().mockResolvedValue(ok({ links: {} }));
    await fetchBacklink(PERMALINK, fetchFn as never);
    await fetchBacklink(PERMALINK, fetchFn as never);
    expect(fetchFn).toHaveBeenCalledTimes(1);
  });

  it("is null and silent when constellation fails — a feed must still render", async () => {
    for (const bad of [
      vi.fn().mockRejectedValue(new Error("offline")),
      vi.fn().mockResolvedValue({ ok: false } as Response),
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => {
          throw new Error("bad json");
        },
      }),
    ]) {
      _resetBacklinkCache();
      await expect(fetchBacklink(PERMALINK, bad as never)).resolves.toBeNull();
    }
  });

  it("does not cache a failure, so a transient outage can recover", async () => {
    const fetchFn = vi
      .fn()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(ok({ links: {} }));
    await fetchBacklink(PERMALINK, fetchFn as never);
    await fetchBacklink(PERMALINK, fetchFn as never);
    expect(fetchFn).toHaveBeenCalledTimes(2);
  });
});

describe("postLink", () => {
  it("opens the post in the reader's chosen client", () => {
    expect(postLink("bsky", DID, "3abc")).toBe(`https://bsky.app/profile/${DID}/post/3abc`);
    expect(postLink("witchsky", DID, "3abc")).toBe(`https://witchsky.app/profile/${DID}/post/3abc`);
  });

  it("defaults to bluesky for an unknown or absent preference", () => {
    // the global feed is public, so there is often no preference at all
    expect(postLink(null, DID, "3abc")).toBe(`https://bsky.app/profile/${DID}/post/3abc`);
    expect(postLink("nonsense", DID, "3abc")).toBe(`https://bsky.app/profile/${DID}/post/3abc`);
  });

  it("sends a record-oriented client to the record instead of a post view", () => {
    // pdsls has no feed ui, so a /profile/../post/.. url would not exist
    expect(postLink("pdsls", DID, "3abc")).toBe(
      `https://pdsls.dev/at/${DID}/app.bsky.feed.post/3abc`,
    );
  });
});
