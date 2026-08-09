/**
 * Rendering and searching gifs, across every registered source.
 *
 * Source-specific knowledge lives in `gifsources.ts`; nothing here names a
 * collection. Everything degrades to "no gif" rather than a broken image, since
 * these schemas are inferred from records in the wild and can change.
 */

import { GIF_SOURCES, parseAtUri, sourceForCollection, sourceForId } from "./gifsources.ts";

export type { GifPost, GifRef, GifVariant } from "./gifsources.ts";
import type { GifPost, GifRef, GifVariant } from "./gifsources.ts";

/**
 * Blob bytes come from our porxie instance (codeberg.org/Blooym/porxie), which
 * fetches from the owning PDS, **verifies the bytes against the CID**, and
 * rejects anything that does not match. Its route is `/{did}/{cid}`.
 *
 * Deliberately not bsky's CDN: it transcodes to jpeg, so it cannot serve an
 * animated gif, and its bytes can never hash to the blob's CID — which would
 * leave the cid in our strongRef an integrity claim nothing ever checks.
 */
// Custom domain, not the *.fly.dev hostname: a provider's default domain
// bypasses the CDN entirely (the same trap plyr.fm hit with r2.dev). Behind
// this, Cloudflare caches the immutable, content-addressed bytes at the edge.
const BLOB_PROXY = "https://porxie.waow.tech";

export function gifBlobUrl(did: string, blobCid: string): string {
  return `${BLOB_PROXY}/${encodeURIComponent(did)}/${encodeURIComponent(blobCid)}`;
}

/**
 * Everything needed to render, derived from the reference alone.
 *
 * Accepts either shape, because the two directions disagree: we *write* a proper
 * strongRef `{uri, cid}` to the PDS, but hatk collapses it to a bare uri string
 * when hydrating a view. Rendering never needs the record's cid — the blob's cid
 * comes from the source's rkey shortcut.
 *
 * Returns null for a source with no shortcut, so callers fall back to the
 * catalog rather than rendering something broken.
 */
export function gifFromRef(ref: GifRef | string | null | undefined): {
  did: string;
  rkey: string;
  blobCid: string;
  /** which source owns it, so display can prefer that source's CDN */
  source: string;
} | null {
  const uri = typeof ref === "string" ? ref : ref?.uri;
  if (!uri) return null;
  const parsed = parseAtUri(uri);
  if (!parsed) return null;

  const source = sourceForCollection(parsed.collection);
  const blobCid = source?.blobCidFromRkey?.(parsed.rkey);
  if (!blobCid || !source) return null;

  return { did: parsed.did, rkey: parsed.rkey, blobCid, source: source.id };
}

/**
 * The url to *display* a gif at.
 *
 * Prefers the owning source's CDN, which serves a far smaller re-encode, and
 * falls back to the CID-verifying blob proxy for any source without one. Not
 * for link previews — see `gifPreviewUrl`.
 */
export function gifRenditionUrl(
  gif: { did: string; blobCid: string; source: string },
  variant: GifVariant = "preview",
): string {
  const source = sourceForId(gif.source);
  return source?.renditionUrl?.(gif.did, gif.blobCid, variant) ?? gifBlobUrl(gif.did, gif.blobCid);
}

/** A publicly fetchable image url for a saved gif — used for link previews. */
export function gifPreviewUrl(ref: GifRef | string | null | undefined): string | null {
  const g = gifFromRef(ref);
  return g ? gifBlobUrl(g.did, g.blobCid) : null;
}

const titleCache = new Map<string, string>();
const MAX_TITLE_CACHE = 300;

/**
 * A gif's own name, for a tooltip on a status that uses it.
 *
 * The status record stores only a strongRef, so the name lives on the source
 * record — which we already index, making this a local lookup rather than a
 * trip to someone's PDS. Falls back to alt text, then tags, since a good number
 * of gifdex records carry no title at all.
 *
 * Bounded, like every other cache here: a feed is not assumed to be small.
 */
export async function fetchGifTitle(callXrpc: XrpcFn, uri: string): Promise<string | null> {
  if (!uri) return null;
  const hit = titleCache.get(uri);
  if (hit !== undefined) return hit || null;

  let label = "";
  try {
    const res = (await callXrpc("dev.hatk.getRecord", { uri })) as
      | { record?: { title?: unknown; tags?: unknown; media?: { alt?: unknown } } }
      | undefined;
    const v = res?.record;
    const tags = Array.isArray(v?.tags)
      ? v.tags.filter((t): t is string => typeof t === "string").join(", ")
      : "";
    label =
      (typeof v?.title === "string" && v.title.trim()) ||
      (typeof v?.media?.alt === "string" && v.media.alt.trim()) ||
      tags;
  } catch {
    // no tooltip is fine; the gif still renders
    return null;
  }

  if (titleCache.size >= MAX_TITLE_CACHE) {
    const oldest = titleCache.keys().next().value;
    if (oldest !== undefined) titleCache.delete(oldest);
  }
  titleCache.set(uri, label);
  return label || null;
}

/** One page of gifs, and the cursor to ask for the next. */
export type GifPage = { gifs: GifPost[]; cursor?: string };

type XrpcFn = (nsid: string, params: Record<string, unknown>) => Promise<unknown>;

type IndexedRecord = { uri?: string; cid?: string; did?: string; [k: string]: unknown };

/**
 * Normalise whatever the index returns into GifPosts, via the owning source.
 *
 * hatk hands back the indexed row rather than the original record, so `media`
 * arrives as parsed JSON under the same key; each source's fromRecord already
 * reads it defensively.
 */
function toPosts(items: unknown[], collection: string): GifPost[] {
  const source = sourceForCollection(collection);
  if (!source) return [];
  const out: GifPost[] = [];
  for (const item of items) {
    const r = item as IndexedRecord;
    const uri = typeof r.uri === "string" ? r.uri : "";
    const parsed = parseAtUri(uri);
    if (!parsed) continue;
    const post = source.fromRecord({
      uri,
      cid: typeof r.cid === "string" ? r.cid : "",
      did: parsed.did,
      rkey: parsed.rkey,
      value: r as Record<string, unknown>,
    });
    if (post) out.push(post);
  }
  return out;
}

/**
 * Browse or search gifs, one page at a time.
 *
 * Everything is paginated and server-side. The catalog is assumed to be
 * unbounded — nothing here holds more than the page being shown, and search
 * runs against hatk's full-text index rather than an array we shipped to the
 * client.
 *
 * Only the first registered source is queried per call; when a second exists
 * this fans out per source and merges, which is why the return shape is a page
 * rather than a raw response.
 */
export async function fetchGifPage(
  callXrpc: XrpcFn,
  opts: { query?: string; cursor?: string; limit?: number } = {},
): Promise<GifPage> {
  const source = GIF_SOURCES[0];
  if (!source) return { gifs: [] };

  const q = opts.query?.trim();
  const limit = opts.limit ?? 30;

  const res = (await callXrpc(
    q ? "dev.hatk.searchRecords" : "dev.hatk.getRecords",
    q
      ? { collection: source.collection, q, limit, cursor: opts.cursor }
      : {
          collection: source.collection,
          limit,
          cursor: opts.cursor,
          sort: "indexed_at",
          order: "DESC",
        },
  )) as { items?: unknown[]; cursor?: string } | undefined;

  return {
    gifs: toPosts(res?.items ?? [], source.collection),
    cursor: res?.cursor,
  };
}
