import { defineFeed } from "$hatk";
import { hydrateStatuses } from "./_hydrate.ts";

// The picker only needs the emoji and its rank, but a feed speaks in record
// URIs — so return the most recent status *using* each emoji, ordered by how
// often that emoji has ever been used. The client reads `.emoji` off each item
// and ignores the rest.
export default defineFeed({
  collection: "io.zzstoatzz.status.record",
  label: "Popular Emoji",

  hydrate: hydrateStatuses,

  async generate(ctx) {
    // no cursor: this is a fixed-size leaderboard, not a scrollable feed.
    const rows = (await ctx.db.query(
      `SELECT t.uri
       FROM "io.zzstoatzz.status.record" t
       JOIN (
         SELECT emoji, COUNT(*) AS uses, MAX(created_at) AS latest
         FROM "io.zzstoatzz.status.record" s
         LEFT JOIN _repos r ON s.did = r.did
         WHERE (r.status IS NULL OR r.status != 'takendown')
         GROUP BY emoji
       ) agg ON agg.emoji = t.emoji AND agg.latest = t.created_at
       LEFT JOIN _repos r2 ON t.did = r2.did
       WHERE (r2.status IS NULL OR r2.status != 'takendown')
       GROUP BY t.emoji
       ORDER BY agg.uses DESC, t.created_at DESC
       LIMIT $1`,
      [ctx.limit],
    )) as { uri: string }[];

    return ctx.ok({ uris: rows.map((r) => r.uri), cursor: undefined });
  },
});
