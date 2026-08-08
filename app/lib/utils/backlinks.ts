/**
 * "Who on bluesky linked to this status?", via constellation.
 *
 * constellation indexes every link in the network by target, so a status
 * permalink is a queryable target like any record. Two shapes reach us: a link
 * card (`.embed.external.uri`) and an inline link in post text
 * (`.facets[].features[app.bsky.richtext.facet#link].uri`), and the exact facet
 * path varies — so paths are discovered rather than hardcoded.
 *
 * The target is the permalink byte-for-byte: constellation treats
 * `…/status/did/rkey` and `…/status/did/rkey/` as different targets, so this
 * must match what the copy-link button produces.
 */

import { parseStatusUri } from "./emoji.ts";

const CONSTELLATION = "https://constellation.microcosm.blue";

/**
 * The canonical permalink for a status.
 *
 * One definition, because the copy-link button and the backlink lookup must
 * produce the same string — constellation matches targets exactly, so even a
 * trailing slash would look like a different page.
 */
export function statusPermalink(origin: string, uri: string): string {
  const { did, rkey } = parseStatusUri(uri);
  return `${origin}/status/${did}/${rkey}`;
}

/**
 * constellation asks callers to identify themselves in the user-agent, but this
 * runs in the browser, where `User-Agent` is a forbidden header and is dropped
 * silently. The cross-origin `Origin: https://status.zzstoatzz.io` we do send
 * identifies us just as well.
 */
const BSKY_POST = "app.bsky.feed.post";

export type Backlink = {
  /** how many bluesky posts reference this status */
  count: number;
  /** the most recent referencing post, for the outbound link */
  did: string;
  rkey: string;
};

/** `{collection: {path: {records, distinct_dids}}}` */
type LinksAll = { links?: Record<string, Record<string, { records?: number }>> };

type FetchLike = typeof globalThis.fetch;

/**
 * Bluesky post paths that actually have links, most-linked first.
 *
 * A path with zero records is reported by constellation but is not worth a
 * follow-up request.
 */
export function bskyPaths(body: LinksAll): { path: string; records: number }[] {
  const paths = body.links?.[BSKY_POST] ?? {};
  return Object.entries(paths)
    .map(([path, v]) => ({ path, records: v?.records ?? 0 }))
    .filter((p) => p.records > 0)
    .sort((a, b) => b.records - a.records);
}

/** `.embed.external.uri` → `app.bsky.feed.post:embed.external.uri` */
export function sourceParam(path: string): string {
  return `${BSKY_POST}:${path.replace(/^\./, "")}`;
}

/**
 * The newest of a set of records.
 *
 * rkeys are TIDs, which sort lexicographically in creation order, so this needs
 * no extra fetch and does not depend on constellation's result ordering.
 */
export function newest<T extends { rkey: string }>(records: T[]): T | null {
  let best: T | null = null;
  for (const r of records) {
    if (!r?.rkey) continue;
    if (!best || r.rkey > best.rkey) best = r;
  }
  return best;
}

/**
 * Bounded so a long feed cannot grow this without limit — the same reason the
 * gif catalog is never held in memory. Oldest insertion is evicted first.
 */
const MAX_CACHE = 300;
const cache = new Map<string, Backlink | null>();

function remember(key: string, value: Backlink | null): Backlink | null {
  if (cache.size >= MAX_CACHE) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
  cache.set(key, value);
  return value;
}

/** Test seam. */
export function _resetBacklinkCache(): void {
  cache.clear();
}

/**
 * Look up bluesky posts referencing a status permalink.
 *
 * One request for the common case — a status nobody has linked — and a second
 * only when there is something to link to. Returns null on any failure: a
 * backlink is decoration, and constellation being down must not break a feed.
 */
export async function fetchBacklink(
  permalink: string,
  fetchFn: FetchLike = globalThis.fetch,
): Promise<Backlink | null> {
  if (cache.has(permalink)) return cache.get(permalink) ?? null;

  try {
    const all = await fetchFn(`${CONSTELLATION}/links/all?target=${encodeURIComponent(permalink)}`);
    if (!all.ok) return null;

    const paths = bskyPaths((await all.json()) as LinksAll);
    if (paths.length === 0) return remember(permalink, null);

    const count = paths.reduce((n, p) => n + p.records, 0);

    const params = new URLSearchParams({
      subject: permalink,
      source: sourceParam(paths[0].path),
      limit: "10",
    });
    const res = await fetchFn(`${CONSTELLATION}/xrpc/blue.microcosm.links.getBacklinks?${params}`);
    if (!res.ok) return null;

    const body = (await res.json()) as { records?: { did: string; rkey: string }[] };
    const latest = newest(body.records ?? []);
    if (!latest) return remember(permalink, null);

    return remember(permalink, { count, did: latest.did, rkey: latest.rkey });
  } catch {
    // offline, blocked, malformed — the status still renders
    return null;
  }
}
