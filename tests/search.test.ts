/**
 * Guards the search patch in `patches/@hatk+hatk+0.0.1-alpha.45.patch`.
 *
 * hatk handed the raw query to FTS5 after stripping `*`, which makes FTS5 match
 * whole tokens only — "cat" missed "catgirl" and a half-typed word matched
 * nothing. The typo-tolerant fallback that would have hidden this is gated on
 * `dialect.jaroWinklerSimilarity`, which only DuckDB provides, so on SQLite
 * there was nothing behind it.
 *
 * Imported by path because hatk does not export this subpath; that is the point
 * of the test — if a reinstall drops the patch, this fails.
 */
import Database from "better-sqlite3";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
// eslint-disable-next-line import/no-relative-packages
import { buildPrefixMatch } from "../node_modules/@hatk/hatk/dist/database/adapters/sqlite-search.js";

describe("buildPrefixMatch", () => {
  it("makes every token a prefix match", () => {
    expect(buildPrefixMatch("cat")).toBe('"cat"*');
  });

  it("ANDs tokens, so more typing narrows rather than widens", () => {
    expect(buildPrefixMatch("head pat")).toBe('"head"* AND "pat"*');
  });

  it("collapses whitespace", () => {
    expect(buildPrefixMatch("  head   pat  ")).toBe('"head"* AND "pat"*');
  });

  it("is empty for a query with no usable tokens", () => {
    for (const q of ["", "   ", '""', "***", null, undefined]) {
      expect(buildPrefixMatch(q as string)).toBe("");
    }
  });

  it("neutralises FTS5 operator syntax instead of letting it through", () => {
    // a user typing punctuation must not produce an fts5 syntax error
    for (const q of ['a"b', "a:b", "a^b", "a(b)", "a*b", "a~b", "a[b]", "a{b}", "a\\b"]) {
      expect(buildPrefixMatch(q)).toBe('"a"* AND "b"*');
    }
  });
});

/**
 * The same queries against real FTS5, on a shadow table shaped like the one hatk
 * builds for `net.gifdex.gif.post`: title is a TEXT column, tags is a child
 * table folded in via string_agg, media_alt is derived from the JSON object, and
 * handle is joined from _repos.
 */
describe("gifdex search against real fts5", () => {
  let db: Database.Database;

  const rows = [
    {
      uri: "at://did:plc:a/net.gifdex.gif.post/1",
      title: "Qualidea Code Head Pat",
      tags_text: "anime catgirl cat headpat pat pet",
      media_alt: "An anime catgirl with silver hair is receiving head pats",
      handle: "olaren.dev",
    },
    {
      uri: "at://did:plc:b/net.gifdex.gif.post/2",
      title: "Bunny kisses",
      tags_text: "bunny rabbit cute",
      media_alt: "",
      handle: "blooym.dev",
    },
    {
      uri: "at://did:plc:c/net.gifdex.gif.post/3",
      title: "Cat loaf",
      tags_text: "cat sitting",
      media_alt: "a cat sitting like a loaf of bread",
      handle: "blooym.dev",
    },
  ];

  const search = (q: string): string[] => {
    const match = buildPrefixMatch(q);
    if (!match) return [];
    return db
      .prepare(`SELECT uri FROM t_fts WHERE t_fts MATCH ? ORDER BY -bm25(t_fts) DESC`)
      .all(match)
      .map((r) => (r as { uri: string }).uri);
  };

  beforeAll(() => {
    db = new Database(":memory:");
    db.exec(`CREATE VIRTUAL TABLE t_fts USING fts5(uri, title, tags_text, media_alt, handle)`);
    const ins = db.prepare(
      `INSERT INTO t_fts (uri, title, tags_text, media_alt, handle) VALUES (?, ?, ?, ?, ?)`,
    );
    for (const r of rows) ins.run(r.uri, r.title, r.tags_text, r.media_alt, r.handle);
  });

  afterAll(() => db.close());

  it("matches a partial word — the case that was entirely broken", () => {
    // "headp" is not a token in any document; only a prefix query finds it
    expect(search("headp")).toEqual(["at://did:plc:a/net.gifdex.gif.post/1"]);
    expect(search("bunn")).toEqual(["at://did:plc:b/net.gifdex.gif.post/2"]);
  });

  it("matches a token that is a prefix of a longer one", () => {
    // "cat" previously could not reach "catgirl"; here it reaches both records
    expect(search("cat").sort()).toEqual([
      "at://did:plc:a/net.gifdex.gif.post/1",
      "at://did:plc:c/net.gifdex.gif.post/3",
    ]);
  });

  it("searches alt text, which the lexicon previously did not declare", () => {
    expect(search("silver")).toEqual(["at://did:plc:a/net.gifdex.gif.post/1"]);
    expect(search("bread")).toEqual(["at://did:plc:c/net.gifdex.gif.post/3"]);
  });

  it("searches tags and handles too", () => {
    expect(search("rabbit")).toEqual(["at://did:plc:b/net.gifdex.gif.post/2"]);
    expect(search("blooym").sort()).toEqual([
      "at://did:plc:b/net.gifdex.gif.post/2",
      "at://did:plc:c/net.gifdex.gif.post/3",
    ]);
  });

  it("narrows as more words are typed", () => {
    expect(search("cat")).toHaveLength(2);
    expect(search("cat loaf")).toEqual(["at://did:plc:c/net.gifdex.gif.post/3"]);
  });

  it("matches across fields, not only within one", () => {
    // "anime" is a tag, "silver" is alt text, on the same record
    expect(search("anime silver")).toEqual(["at://did:plc:a/net.gifdex.gif.post/1"]);
  });

  it("still finds nothing for a genuine miss", () => {
    expect(search("dinosaur")).toEqual([]);
    expect(search("cat dinosaur")).toEqual([]);
  });

  it("does not throw on punctuation a user might type", () => {
    for (const q of ['cat"', "cat:", "cat*", "(cat)", "^cat", "cat~1"]) {
      expect(() => search(q)).not.toThrow();
    }
  });

  it("is case insensitive", () => {
    expect(search("CATGIRL")).toEqual(["at://did:plc:a/net.gifdex.gif.post/1"]);
  });
});
