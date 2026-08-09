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

/** how many link paths to follow per status; in practice there are one or two */
const MAX_PATHS = 3;

/** one bluesky post that references a status */
export type BacklinkPost = { did: string; rkey: string };

export type Backlink = {
  /** how many bluesky posts reference this status, across every link path */
  count: number;
  /** every post we could resolve, newest first, so the reader can pick one */
  posts: BacklinkPost[];
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
 * Records newest first, dropping anything unusable.
 *
 * rkeys are TIDs, which sort lexicographically in creation order, so ordering
 * needs no extra fetch and does not depend on constellation's result order.
 */
export function newestFirst<T extends { rkey: string; did: string }>(records: T[]): T[] {
  return records.filter((r) => r?.rkey && r?.did).sort((a, b) => (a.rkey > b.rkey ? -1 : 1));
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
  handleCache.clear();
}

const handleCache = new Map<string, string>();

/**
 * Handles for the backlink menu, so it reads as people rather than DIDs.
 *
 * One request for the whole set — the public appview takes up to 25 actors at
 * a time, which is the same ceiling we ask constellation for. Only called when
 * a menu is opened, so most statuses never trigger it. Anything unresolved is
 * simply absent, and the caller falls back to a truncated DID.
 */
export async function resolveHandles(
  dids: string[],
  fetchFn: FetchLike = globalThis.fetch,
): Promise<Record<string, string>> {
  const unique = [...new Set(dids)].filter(Boolean);
  const out: Record<string, string> = {};
  const missing: string[] = [];
  for (const did of unique) {
    const hit = handleCache.get(did);
    if (hit) out[did] = hit;
    else missing.push(did);
  }
  if (missing.length === 0) return out;

  try {
    const params = new URLSearchParams();
    for (const did of missing.slice(0, 25)) params.append("actors", did);
    const res = await fetchFn(
      `https://public.api.bsky.app/xrpc/app.bsky.actor.getProfiles?${params}`,
    );
    if (!res.ok) return out;
    const body = (await res.json()) as { profiles?: { did: string; handle: string }[] };
    for (const p of body.profiles ?? []) {
      if (!p?.did || !p?.handle) continue;
      handleCache.set(p.did, p.handle);
      out[p.did] = p.handle;
    }
  } catch {
    // a menu of truncated DIDs still works
  }
  return out;
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

    // Union every path, rather than summing their counts. A post that both
    // links inline and carries a link card to the same url is indexed under two
    // paths, so summing double-counts it — checked against real records, where
    // every "2" turned out to be one post seen twice. The count has to be
    // distinct posts, or the badge disagrees with the menu it opens.
    const byKey = new Map<string, BacklinkPost>();
    for (const { path } of paths.slice(0, MAX_PATHS)) {
      const params = new URLSearchParams({
        subject: permalink,
        source: sourceParam(path),
        // enough to offer a real choice without paging; the tail is rare
        limit: "25",
      });
      const res = await fetchFn(
        `${CONSTELLATION}/xrpc/blue.microcosm.links.getBacklinks?${params}`,
      );
      if (!res.ok) continue;
      const body = (await res.json()) as { records?: BacklinkPost[] };
      for (const r of body.records ?? []) {
        if (r?.did && r?.rkey) byKey.set(`${r.did}/${r.rkey}`, { did: r.did, rkey: r.rkey });
      }
    }

    const posts = newestFirst([...byKey.values()]);
    if (posts.length === 0) return remember(permalink, null);

    return remember(permalink, { count: posts.length, posts });
  } catch {
    // offline, blocked, malformed — the status still renders
    return null;
  }
}
