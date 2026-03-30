import { redirect } from "@sveltejs/kit";
import { callXrpc } from "$hatk/client";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({ params }) => {
  const handle = decodeURIComponent(params.handle);

  try {
    const res = await callXrpc("dev.hatk.getFeed", {
      feed: "actor",
      actor: handle,
      limit: 1,
    });
    const did = res.items?.[0]?.did;
    if (did) {
      redirect(302, `/profile/${encodeURIComponent(did)}`);
    }
  } catch {}

  // no statuses found — redirect anyway, profile page will show "no statuses yet"
  redirect(302, `/profile/${encodeURIComponent(handle)}`);
};
