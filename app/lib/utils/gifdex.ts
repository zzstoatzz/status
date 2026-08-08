/**
 * gifdex (net.gifdex.*) support.
 *
 * There is no gifdex instance — gifdex.net does not resolve, and no lexicon is
 * published for the NSID authority. Everything here is inferred from records in
 * the wild, so it stays tolerant: anything unexpected degrades to "no gif" and
 * the status falls back to its emoji.
 */

export const GIFDEX_POST = "net.gifdex.gif.post";

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
};

/** at://<did>/<collection>/<rkey> — returns null for anything else. */
export function parseAtUri(uri: string): { did: string; collection: string; rkey: string } | null {
  const m = /^at:\/\/([^/]+)\/([^/]+)\/(.+)$/.exec(uri ?? "");
  if (!m) return null;
  return { did: m[1], collection: m[2], rkey: m[3] };
}

/**
 * gifdex rkeys are `<tid>:<blobCid>`, and the embedded cid is the media blob's
 * — verified across every record sampled from every repo publishing them. That
 * makes a gif renderable straight from its at-uri with no record fetch at all.
 *
 * Returns null when the rkey does not carry a cid, so callers can fall back to
 * reading the record rather than rendering something broken.
 */
export function blobCidFromRkey(rkey: string): string | null {
  const idx = rkey.indexOf(":");
  if (idx <= 0) return null;
  const cid = rkey.slice(idx + 1);
  // base32 CIDv1 — "b" prefix, lowercase alphanumeric. Guards against a rkey
  // that merely happens to contain a colon.
  return /^b[a-z2-7]{20,}$/.test(cid) ? cid : null;
}

/**
 * Blob bytes come from our porxie instance (codeberg.org/Blooym/porxie), which
 * fetches from the owning PDS, **verifies the bytes against the CID**, and
 * rejects anything that does not match. Its route is `/{did}/{cid}`, which is
 * exactly what a gifdex rkey already gives us — so no PDS resolution is needed
 * here at all.
 *
 * Deliberately not bsky's CDN: it transcodes to jpeg, so it cannot serve an
 * animated gif, and its bytes can never hash to the blob's CID — which would
 * leave the cid in our strongRef an integrity claim nothing ever checks.
 */
const BLOB_PROXY = "https://zzstoatzz-porxie.fly.dev";

export function gifBlobUrl(did: string, blobCid: string): string {
  return `${BLOB_PROXY}/${encodeURIComponent(did)}/${encodeURIComponent(blobCid)}`;
}

/**
 * Everything needed to render, derived from the reference alone.
 *
 * Accepts either shape, because the two directions disagree: we *write* a proper
 * strongRef `{uri, cid}` to the PDS, but hatk collapses it to a bare uri string
 * when hydrating a view (the cid lives in its own column and is not surfaced).
 * Rendering never needs the cid — the blob's cid is in the rkey.
 */
export function gifFromRef(ref: GifRef | string | null | undefined): {
  did: string;
  rkey: string;
  blobCid: string;
} | null {
  const uri = typeof ref === "string" ? ref : ref?.uri;
  if (!uri) return null;
  const parsed = parseAtUri(uri);
  if (!parsed || parsed.collection !== GIFDEX_POST) return null;
  const blobCid = blobCidFromRkey(parsed.rkey);
  if (!blobCid) return null;
  return { did: parsed.did, rkey: parsed.rkey, blobCid };
}

/** Case-insensitive match over title and tags. */
export function searchGifs(query: string, gifs: GifPost[]): GifPost[] {
  const q = query.trim().toLowerCase();
  if (!q) return gifs;
  const terms = q.split(/\s+/);
  return gifs.filter((g) => {
    const haystack = `${g.title ?? ""} ${g.tags.join(" ")}`.toLowerCase();
    return terms.every((t) => haystack.includes(t));
  });
}

let catalogCache: GifPost[] | null = null;

/** The whole gifdex catalog — small enough (~150 records) to hold in memory. */
export async function loadGifCatalog(fetchFn: typeof fetch = fetch): Promise<GifPost[]> {
  if (catalogCache) return catalogCache;
  const res = await fetchFn("/api/gifs");
  if (!res.ok) throw new Error("failed to load gif catalog");
  const data = (await res.json()) as { gifs?: GifPost[] };
  catalogCache = data.gifs ?? [];
  return catalogCache;
}
