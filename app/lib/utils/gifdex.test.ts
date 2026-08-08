import { describe, expect, it } from "vitest";
import {
  blobCidFromRkey,
  gifFromRef,
  parseAtUri,
  gifBlobUrl,
  searchGifs,
  type GifPost,
} from "./gifdex.ts";

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

describe("blobCidFromRkey", () => {
  // gifdex rkeys are `<tid>:<blobCid>`, verified across every repo publishing
  // them — which is what lets a gif render with no record fetch at all.
  it("extracts the blob cid embedded in the rkey", () => {
    expect(blobCidFromRkey(RKEY)).toBe(BLOB);
  });

  it("returns null for a plain tid rkey", () => {
    // gif.save records use bare tids; a future gif.post might too
    expect(blobCidFromRkey("3mrubv4zyvs2f")).toBeNull();
  });

  it("rejects a colon that is not followed by a cid", () => {
    for (const bad of ["tid:", "tid:nope", "tid:zzz-not-base32", ":leading"]) {
      expect(blobCidFromRkey(bad)).toBeNull();
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

describe("searchGifs", () => {
  const gifs: GifPost[] = [
    {
      uri: "at://a/net.gifdex.gif.post/1:b",
      cid: "c",
      did: "a",
      rkey: "1:b",
      blobCid: "b",
      title: "Bunny kisses",
      tags: ["bunny", "cuddle", "love"],
    },
    {
      uri: "at://a/net.gifdex.gif.post/2:b",
      cid: "c",
      did: "a",
      rkey: "2:b",
      blobCid: "b",
      title: "Freedom",
      tags: ["puppygirl", "arf", "discord"],
    },
    {
      uri: "at://a/net.gifdex.gif.post/3:b",
      cid: "c",
      did: "a",
      rkey: "3:b",
      blobCid: "b",
      tags: ["bocchi"],
    },
  ];

  it("matches on title and on tags", () => {
    expect(searchGifs("bunny", gifs).map((g) => g.title)).toEqual(["Bunny kisses"]);
    expect(searchGifs("discord", gifs).map((g) => g.title)).toEqual(["Freedom"]);
  });

  it("is case-insensitive", () => {
    expect(searchGifs("FREEDOM", gifs)).toHaveLength(1);
  });

  it("requires every term to match, across title and tags together", () => {
    expect(searchGifs("bunny love", gifs)).toHaveLength(1);
    expect(searchGifs("bunny discord", gifs)).toHaveLength(0);
  });

  it("tolerates a record with no title", () => {
    expect(searchGifs("bocchi", gifs)).toHaveLength(1);
  });

  it("returns everything for an empty query", () => {
    expect(searchGifs("   ", gifs)).toHaveLength(3);
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
