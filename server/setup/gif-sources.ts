import { defineSetup } from "@hatk/hatk/setup";
import { GIF_SOURCES } from "../../app/lib/utils/gifsources.ts";

const LIGHTRAIL = "https://lightrail.microcosm.blue";

/**
 * Seed the repos that publish gifs, so backfill ingests what already exists.
 *
 * The indexer only learns about a repo when it sees a live commit, which would
 * leave every gif posted before we started indexing invisible. lightrail answers
 * "who has records in this collection" directly, so we insert those DIDs as
 * pending and let the normal backfill path pull them in.
 *
 * Runs for every registered source, so adding a source needs no change here.
 * Best-effort: if lightrail is unreachable the app still boots, and live commits
 * will discover repos over time regardless.
 */
export default defineSetup(async ({ db }) => {
  await db.run(`
    CREATE INDEX IF NOT EXISTS idx_gifdex_indexed
    ON "net.gifdex.gif.post" (indexed_at DESC)
  `);

  for (const source of GIF_SOURCES) {
    try {
      const res = await fetch(
        `${LIGHTRAIL}/xrpc/com.atproto.sync.listReposByCollection?collection=${source.collection}&limit=500`,
      );
      if (!res.ok) continue;
      const data = (await res.json()) as { repos?: { did: string }[] };
      for (const { did } of data.repos ?? []) {
        // OR IGNORE: never disturb a repo we already track
        await db.run(`INSERT OR IGNORE INTO _repos (did, status) VALUES ($1, 'pending')`, [did]);
      }
    } catch {
      // lightrail down — live commits will still discover repos
    }
  }
});
