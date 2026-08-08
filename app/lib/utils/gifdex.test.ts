import { describe, expect, it } from "vitest";
import { fetchGifPage, gifBlobUrl, gifFromRef, gifPreviewUrl } from "./gifdex.ts";
import { gifdexBlobCidFromRkey, parseAtUri } from "./gifsources.ts";
import { vi } from "vitest";

// a real record, from blooym.dev's repo
const DID = "did:plc:p5yjdr64h7mk5l3kh6oszryk";
const BLOB = "bafkreifh2wqcrow6ip3fvln5c2h54ymvtaunrf2tbgpu3b2ty7y2pvr2r4";
const RKEY = `3mrxc4gdo5s2x:${BLOB}`;
const URI = `at://${DID}/net.gifdex.gif.post/${RKEY}`;

describe("parseAtUri", () => {
  it("splits a gifdex uri, keeping the colon in the rkey", () => {
    expect(parseAtUri(URI)).toEqual({
      did: DID,
      collection: "net.gifdex.gif.post",
      rkey: RKEY,
    });
  });

  it("returns null for non-at uris", () => {
    for (const bad of ["", "https://example.com", "at://did:plc:x", "not a uri"]) {
      expect(parseAtUri(bad)).toBeNull();
    }
  });
});

describe("gifdexBlobCidFromRkey", () => {
  // gifdex rkeys are `<tid>:<blobCid>`, verified across every repo publishing
  // them — which is what lets a gif render with no record fetch at all.
  it("extracts the blob cid embedded in the rkey", () => {
    expect(gifdexBlobCidFromRkey(RKEY)).toBe(BLOB);
  });

  it("returns null for a plain tid rkey", () => {
    // gif.save records use bare tids; a future gif.post might too
    expect(gifdexBlobCidFromRkey("3mrubv4zyvs2f")).toBeNull();
  });

  it("rejects a colon that is not followed by a cid", () => {
    for (const bad of ["tid:", "tid:nope", "tid:zzz-not-base32", ":leading"]) {
      expect(gifdexBlobCidFromRkey(bad)).toBeNull();
    }
  });
});

describe("gifFromRef", () => {
  it("derives did and blob cid from a strongRef alone", () => {
    expect(gifFromRef({ uri: URI, cid: "bafyreiwhatever" })).toEqual({
      did: DID,
      rkey: RKEY,
      blobCid: BLOB,
    });
  });

  it("is null for a missing or malformed ref, so callers fall back to the emoji", () => {
    expect(gifFromRef(undefined)).toBeNull();
    expect(gifFromRef(null)).toBeNull();
    expect(gifFromRef({ uri: "", cid: "" })).toBeNull();
  });

  it("refuses a strongRef pointing at some other collection", () => {
    // the field is typed as a generic strongRef, so this is reachable
    const other = `at://${DID}/app.bsky.feed.post/${RKEY}`;
    expect(gifFromRef({ uri: other, cid: "bafy" })).toBeNull();
  });

  it("is null when the rkey carries no cid, rather than rendering a broken image", () => {
    expect(
      gifFromRef({ uri: `at://${DID}/net.gifdex.gif.post/3mrubv4zyvs2f`, cid: "bafy" }),
    ).toBeNull();
  });
});

describe("gifBlobUrl", () => {
  // Never bsky's CDN: it transcodes to jpeg, so it cannot serve an animated gif,
  // and its bytes can never hash to the cid our strongRef claims to pin. porxie
  // fetches from the owning PDS and verifies the bytes against the cid.
  it("builds a porxie /{did}/{cid} url", () => {
    // the did's colons come out percent-encoded; verified live that porxie
    // decodes the segment and serves both forms identically
    expect(gifBlobUrl(DID, BLOB)).toBe(
      `https://zzstoatzz-porxie.fly.dev/${encodeURIComponent(DID)}/${BLOB}`,
    );
  });

  it("encodes each segment, so a malformed did cannot inject a path", () => {
    const url = gifBlobUrl("did:web:evil.com/../../admin", BLOB);
    expect(url).not.toContain("/../");
    expect(url.startsWith("https://zzstoatzz-porxie.fly.dev/")).toBe(true);
    expect(url.split("/").length).toBe(5); // https, '', host, did, cid
  });
});

describe("gifFromRef with hatk's flattened shape", () => {
  // hatk stores a strongRef as {name}_uri + {name}_cid, then hydrates only the
  // uri back into the view — so the read path hands us a bare string even
  // though the write path stored a proper strongRef.
  it("accepts a bare uri string as well as a strongRef", () => {
    expect(gifFromRef(URI)).toEqual({ did: DID, rkey: RKEY, blobCid: BLOB });
    expect(gifFromRef(URI)).toEqual(gifFromRef({ uri: URI, cid: "bafyanything" }));
  });

  it("still rejects a bare string that is not a gifdex post", () => {
    expect(gifFromRef(`at://${DID}/app.bsky.feed.post/${RKEY}`)).toBeNull();
    expect(gifFromRef("")).toBeNull();
  });
});

describe("gifPreviewUrl", () => {
  // a gif status should preview as the gif. porxie serves it CID-verified at a
  // stable public url, and bluesky's card proxy passes gifs through unmodified.
  it("gives a public image url for a saved gif", () => {
    expect(gifPreviewUrl({ uri: URI, cid: "bafyanything" })).toBe(
      `https://zzstoatzz-porxie.fly.dev/${encodeURIComponent(DID)}/${BLOB}`,
    );
  });

  it("is null when there is no gif, so the emoji preview is used instead", () => {
    expect(gifPreviewUrl(undefined)).toBeNull();
    expect(gifPreviewUrl("")).toBeNull();
  });
});

describe("source registry", () => {
  it("resolves a gif only for a registered collection", async () => {
    const { sourceForCollection } = await import("./gifsources.ts");
    expect(sourceForCollection("net.gifdex.gif.post")?.id).toBe("gifdex");
    expect(sourceForCollection("com.example.gif.post")).toBeUndefined();
  });

  // the tid:blobCid rkey trick is gifdex's convention, not a general one — a
  // future source without it must fall back rather than render a broken image
  it("returns null for a collection with no rkey shortcut", () => {
    expect(gifFromRef(`at://${DID}/com.example.gif.post/${RKEY}`)).toBeNull();
  });
});

describe("fetchGifPage", () => {
  const row = {
    uri: URI,
    cid: "bafyrecord",
    media: { blob: { ref: { $link: BLOB } }, dimensions: { width: 360, height: 377 } },
    title: "Bunny kisses",
    tags: ["bunny"],
  };

  // the catalog is assumed unbounded: never fetch it whole, never hold it whole
  it("browses one page at a time and passes the cursor back", async () => {
    const callXrpc = vi.fn(async () => ({ items: [row], cursor: "page2" }));
    const page = await fetchGifPage(callXrpc, { limit: 30 });

    expect(callXrpc).toHaveBeenCalledWith("dev.hatk.getRecords", {
      collection: "net.gifdex.gif.post",
      limit: 30,
      cursor: undefined,
      sort: "indexed_at",
      order: "DESC",
    });
    expect(page.cursor).toBe("page2");
    expect(page.gifs).toHaveLength(1);
    expect(page.gifs[0]).toMatchObject({ did: DID, blobCid: BLOB, title: "Bunny kisses" });
  });

  it("searches through the index rather than filtering locally", async () => {
    const callXrpc = vi.fn(async () => ({ items: [] }));
    await fetchGifPage(callXrpc, { query: "  bunny " });

    expect(callXrpc).toHaveBeenCalledWith("dev.hatk.searchRecords", {
      collection: "net.gifdex.gif.post",
      q: "bunny",
      limit: 30,
      cursor: undefined,
    });
  });

  it("reports exhaustion by omitting a cursor", async () => {
    const callXrpc = vi.fn(async () => ({ items: [row] }));
    await expect(fetchGifPage(callXrpc)).resolves.toMatchObject({ cursor: undefined });
  });

  it("drops rows it cannot render instead of emitting broken tiles", async () => {
    const callXrpc = vi.fn(async () => ({
      items: [row, { uri: "not-an-at-uri" }, { uri: URI.replace(RKEY, "plaintid"), media: {} }],
    }));
    const page = await fetchGifPage(callXrpc);
    expect(page.gifs).toHaveLength(1);
  });
});
