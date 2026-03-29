import { defineFeed } from "$hatk";
import { hydrateStatuses } from "./_hydrate.ts";

export default defineFeed({
  collection: "io.zzstoatzz.status.record",
  label: "Actor Statuses",

  hydrate: hydrateStatuses,

  async generate(ctx) {
    const { params, ok, isTakendown } = ctx;

    let actor = params.actor;
    if (!actor) {
      return ok({ uris: [], cursor: undefined });
    }

    if (!actor.startsWith("did:")) {
      const rows = (await ctx.db.query(
        `SELECT did FROM _repos WHERE handle = $1`,
        [actor],
      )) as { did: string }[];
      if (rows[0]?.did) {
        actor = rows[0].did;
      }
    }

    if (await isTakendown(actor)) {
      return ok({ uris: [], cursor: undefined });
    }

    const { rows, cursor } = await ctx.paginate<{ uri: string }>(
      `SELECT t.uri, t.cid, t.created_at
       FROM "io.zzstoatzz.status.record" t
       WHERE t.did = $1`,
      { params: [actor], orderBy: "t.created_at" },
    );

    return ok({ uris: rows.map((r) => r.uri), cursor });
  },
});
