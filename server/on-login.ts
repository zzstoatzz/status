import { defineHook } from "$hatk";

export default defineHook("on-login", async (ctx) => {
  const { did, ensureRepo } = ctx;
  await ensureRepo(did);
});
