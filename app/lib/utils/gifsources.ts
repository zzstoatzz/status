/**
 * Where gifs come from.
 *
 * gifdex is the first source, not the only one. A source is "some collection
 * whose records point at a gif blob", and everything downstream — the picker
 * tab, the catalog endpoint, link previews — works off this registry rather
 * than any one NSID.
 *
 * Adding a source means adding an entry here. Nothing else should mention a
 * collection by name.
 */

export type GifRef = { uri: string; cid: string };

export type GifPost = {
  uri: string;
  cid: string;
  did: string;
  rkey: string;
  title?: string;
  tags: string[];
  blobCid: string;
  width?: number;
  height?: number;
  /** which source produced this, for credit in the UI */
  source: string;
};

/** The subset of a listRecords row a source needs to build a GifPost. */
export type RawRecord = {
  uri: string;
  cid: string;
  did: string;
  rkey: string;
  value: Record<string, unknown>;
};

export type GifSource = {
  id: string;
  collection: string;
  /** shown as the credit line under the picker grid */
  credit?: { label: string; href: string };
  /**
   * Optional shortcut: derive the media blob's cid from the rkey alone, so a
   * saved gif renders with no record fetch. Only some sources encode it there.
   */
  blobCidFromRkey?: (rkey: string) => string | null;
  /** Build a GifPost from a record, or null if it is not renderable. */
  fromRecord: (raw: RawRecord) => GifPost | null;
};

/** at://<did>/<collection>/<rkey> — null for anything else. */
export function parseAtUri(uri: string): { did: string; collection: string; rkey: string } | null {
  const m = /^at:\/\/([^/]+)\/([^/]+)\/(.+)$/.exec(uri ?? "");
  if (!m) return null;
  return { did: m[1], collection: m[2], rkey: m[3] };
}

/** base32 CIDv1: "b" prefix, lowercase alphanumeric. */
function isCid(s: string): boolean {
  return /^b[a-z2-7]{20,}$/.test(s);
}

/**
 * gifdex rkeys are `<tid>:<blobCid>`, and the embedded cid is the media blob's
 * — verified across every record from every repo publishing them. That is a
 * gifdex convention, not a general one, which is why it lives on the source
 * rather than in shared code.
 */
export function gifdexBlobCidFromRkey(rkey: string): string | null {
  const idx = rkey.indexOf(":");
  if (idx <= 0) return null;
  const cid = rkey.slice(idx + 1);
  return isCid(cid) ? cid : null;
}

/** Pull `media.blob.ref.$link` + dimensions out of a gifdex-shaped record. */
function gifdexFromRecord(raw: RawRecord): GifPost | null {
  const v = raw.value as {
    title?: unknown;
    tags?: unknown;
    media?: {
      blob?: { ref?: { $link?: string } };
      dimensions?: { width?: number; height?: number };
    };
  };
  const blobCid = v.media?.blob?.ref?.$link ?? gifdexBlobCidFromRkey(raw.rkey);
  if (!blobCid) return null;

  return {
    uri: raw.uri,
    cid: raw.cid,
    did: raw.did,
    rkey: raw.rkey,
    title: typeof v.title === "string" ? v.title : undefined,
    tags: Array.isArray(v.tags) ? v.tags.filter((t): t is string => typeof t === "string") : [],
    blobCid,
    width: v.media?.dimensions?.width,
    height: v.media?.dimensions?.height,
    source: "gifdex",
  };
}

export const GIF_SOURCES: GifSource[] = [
  {
    id: "gifdex",
    collection: "net.gifdex.gif.post",
    credit: { label: "gifs from gifdex", href: "https://gifdex.net" },
    blobCidFromRkey: gifdexBlobCidFromRkey,
    fromRecord: gifdexFromRecord,
  },
];

export function sourceForCollection(collection: string): GifSource | undefined {
  return GIF_SOURCES.find((s) => s.collection === collection);
}

export const GIF_COLLECTIONS = GIF_SOURCES.map((s) => s.collection);
