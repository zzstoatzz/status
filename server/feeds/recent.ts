import { defineFeed } from "$hatk";
import { hydrateStatuses } from "./_hydrate.ts";

export default defineFeed({
  collection: "io.zzstoatzz.status.record",
  label: "Recent Statuses",

  hydrate: hydrateStatuses,

  async generate(ctx) {
    const { rows, cursor } = await ctx.paginate<{ uri: string }>(
      `SELECT t.uri, t.cid, t.created_at FROM "io.zzstoatzz.status.record" t
       LEFT JOIN _repos r ON t.did = r.did
       WHERE (r.status IS NULL OR r.status != 'takendown')`,
      { orderBy: "t.created_at" },
    );

    return ctx.ok({ uris: rows.map((r) => r.uri), cursor });
  },
});
