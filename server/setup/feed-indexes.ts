import { defineSetup } from "@hatk/hatk/setup";

export default defineSetup(async ({ db }) => {
  await db.run(`
    CREATE INDEX IF NOT EXISTS idx_status_created
    ON "io.zzstoatzz.status.record" (created_at DESC, cid DESC)
  `);

  await db.run(`
    CREATE INDEX IF NOT EXISTS idx_status_actor_created
    ON "io.zzstoatzz.status.record" (did, created_at DESC, cid DESC)
  `);
});
