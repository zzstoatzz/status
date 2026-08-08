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
  size?: number;
};

/** at://<did>/<collection>/<rkey> — returns null for anything else. */
export function parseAtUri(uri: string): { did: string; collection: string; rkey: string } | null {
  const m = /^at:\/\/([^/]+)\/([^/]+)\/(.+)$/.exec(uri ?? "");
  if (!m) return null;
  return { did: m[1], collection: m[2], rkey: m[3] };
}

export function isGifdexPost(uri: string): boolean {
  return parseAtUri(uri)?.collection === GIFDEX_POST;
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

/** Full-size animated gif, straight from the owning PDS. */
export function gifBlobUrl(pds: string, did: string, blobCid: string): string {
  return `${pds.replace(/\/$/, "")}/xrpc/com.atproto.sync.getBlob?did=${encodeURIComponent(did)}&cid=${encodeURIComponent(blobCid)}`;
}

/**
 * A static thumbnail, two to three orders of magnitude smaller than the gif
 * (measured 9.5MB -> 11KB). Grids use this; only a chosen gif animates.
 *
 * This is bsky's CDN serving a blob from a non-bsky PDS, which is not a
 * documented guarantee — always render it with a full-blob fallback.
 */
export function gifThumbUrl(did: string, blobCid: string): string {
  return `https://cdn.bsky.app/img/feed_thumbnail/plain/${did}/${blobCid}@jpeg`;
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

/** The whole gifdex catalog. Small enough (~153 records) to hold in memory. */
export async function loadGifCatalog(fetchFn: typeof fetch = fetch): Promise<GifPost[]> {
  if (catalogCache) return catalogCache;
  const res = await fetchFn("/api/gifs");
  if (!res.ok) throw new Error("failed to load gif catalog");
  const data = (await res.json()) as { gifs?: GifPost[] };
  catalogCache = data.gifs ?? [];
  return catalogCache;
}

const pdsByDid = new Map<string, Promise<string | null>>();

/**
 * Resolve a DID to its PDS, which is where an animated gif has to come from —
 * the CDN thumbnail is a still frame.
 *
 * Memoized per DID: a feed of gifs from one person must not re-resolve per tile.
 */
export function resolvePdsForDid(
  did: string,
  fetchFn: typeof fetch = fetch,
): Promise<string | null> {
  const cached = pdsByDid.get(did);
  if (cached) return cached;

  const docUrl = did.startsWith("did:web:")
    ? `https://${decodeURIComponent(did.slice("did:web:".length))}/.well-known/did.json`
    : `https://plc.directory/${encodeURIComponent(did)}`;

  const p = fetchFn(docUrl)
    .then((r) => (r.ok ? r.json() : null))
    .then((doc: { service?: { type: string; serviceEndpoint: string }[] } | null) => {
      const endpoint = doc?.service?.find(
        (s) => s.type === "AtprotoPersonalDataServer",
      )?.serviceEndpoint;
      return endpoint ?? null;
    })
    .catch(() => null);

  pdsByDid.set(did, p);
  return p;
}
