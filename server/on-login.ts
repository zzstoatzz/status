import { defineHook } from "$hatk";

export default defineHook("on-login", async (ctx) => {
  const { did, ensureRepo } = ctx;
  void ensureRepo(did).catch((err) => {
    console.error("[on-login] failed to backfill repo", { did, err });
  });
});
