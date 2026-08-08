import { json } from "@sveltejs/kit";
import type { RequestHandler } from "./$types";
import { GIFDEX_POST, blobCidFromRkey, type GifPost } from "$lib/utils/gifdex";

/**
 * The gifdex catalog, aggregated here because gifdex has no appview — gifdex.net
 * does not resolve, so there is nothing to query. We assemble it the way the
 * ecosystem does for a sparse collection: lightrail for "who has records", then
 * listRecords against each owning PDS.
 *
 * That is only viable because the corpus is tiny (~150 records across ~8 repos).
 * If it grows by orders of magnitude this needs to become an indexed collection
 * rather than a fan-out.
 */

const LIGHTRAIL = "https://lightrail.microcosm.blue";
const PLC = "https://plc.directory";

// Long, because the answer barely changes and a cold rebuild fans out to every
// PDS. Served stale-then-refreshed rather than making a user wait on a rebuild.
const TTL_MS = 10 * 60 * 1000;

let cache: { at: number; gifs: GifPost[] } | null = null;
let inFlight: Promise<GifPost[]> | null = null;

const pdsCache = new Map<string, string | null>();

async function resolvePds(did: string, f: typeof fetch): Promise<string | null> {
  if (pdsCache.has(did)) return pdsCache.get(did) ?? null;
  let endpoint: string | null = null;
  try {
    const res = await f(`${PLC}/${encodeURIComponent(did)}`);
    if (res.ok) {
      const doc = (await res.json()) as {
        service?: { type: string; serviceEndpoint: string }[];
      };
      endpoint =
        doc.service?.find((s) => s.type === "AtprotoPersonalDataServer")?.serviceEndpoint ?? null;
    }
  } catch {
    endpoint = null;
  }
  pdsCache.set(did, endpoint);
  return endpoint;
}

async function listRepos(f: typeof fetch): Promise<string[]> {
  const res = await f(
    `${LIGHTRAIL}/xrpc/com.atproto.sync.listReposByCollection?collection=${GIFDEX_POST}&limit=500`,
  );
  if (!res.ok) throw new Error(`lightrail ${res.status}`);
  const data = (await res.json()) as { repos?: { did: string }[] };
  return (data.repos ?? []).map((r) => r.did);
}

type RawRecord = {
  uri: string;
  cid: string;
  value: {
    title?: string;
    tags?: unknown;
    media?: {
      blob?: { ref?: { $link?: string } };
      dimensions?: { width?: number; height?: number };
    };
  };
};

async function listGifs(did: string, f: typeof fetch): Promise<GifPost[]> {
  const pds = await resolvePds(did, f);
  if (!pds) return [];

  const out: GifPost[] = [];
  let cursor: string | undefined;

  // bounded: a runaway or hostile repo must not stall the whole catalog
  for (let page = 0; page < 10; page++) {
    const url = new URL(`${pds.replace(/\/$/, "")}/xrpc/com.atproto.repo.listRecords`);
    url.searchParams.set("repo", did);
    url.searchParams.set("collection", GIFDEX_POST);
    url.searchParams.set("limit", "100");
    if (cursor) url.searchParams.set("cursor", cursor);

    const res = await f(url);
    if (!res.ok) break;
    const data = (await res.json()) as { records?: RawRecord[]; cursor?: string };
    const records = data.records ?? [];

    for (const r of records) {
      const rkey = r.uri.split("/").pop() ?? "";
      // the blob cid lives in the rkey; prefer the record's own ref when present
      const blobCid = r.value.media?.blob?.ref?.$link ?? blobCidFromRkey(rkey);
      if (!blobCid) continue; // unrenderable — skip rather than emit a broken tile

      out.push({
        uri: r.uri,
        cid: r.cid,
        did,
        rkey,
        title: typeof r.value.title === "string" ? r.value.title : undefined,
        tags: Array.isArray(r.value.tags)
          ? r.value.tags.filter((t): t is string => typeof t === "string")
          : [],
        blobCid,
        width: r.value.media?.dimensions?.width,
        height: r.value.media?.dimensions?.height,
      });
    }

    cursor = data.cursor;
    if (!cursor || records.length === 0) break;
  }

  return out;
}

async function buildCatalog(f: typeof fetch): Promise<GifPost[]> {
  const dids = await listRepos(f);
  const perRepo = await Promise.all(
    dids.map((did) => listGifs(did, f).catch(() => [] as GifPost[])),
  );
  // newest first, by the TID that leads each rkey (lexicographically sortable)
  return perRepo.flat().sort((a, b) => b.rkey.localeCompare(a.rkey));
}

export const GET: RequestHandler = async ({ fetch, setHeaders }) => {
  const fresh = cache && Date.now() - cache.at < TTL_MS;

  if (!fresh) {
    // one rebuild at a time, however many requests arrive during it
    inFlight ??= buildCatalog(fetch)
      .then((gifs) => {
        cache = { at: Date.now(), gifs };
        return gifs;
      })
      .finally(() => {
        inFlight = null;
      });

    // serve stale immediately if we have it; only block on a cold cache
    if (!cache) {
      try {
        await inFlight;
      } catch {
        return json({ gifs: [], error: "catalog unavailable" }, { status: 200 });
      }
    }
  }

  setHeaders({ "cache-control": "public, max-age=300" });
  return json({ gifs: cache?.gifs ?? [] });
};
