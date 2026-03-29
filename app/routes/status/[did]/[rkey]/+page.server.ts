import { callXrpc } from "$hatk/client";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({ params }) => {
  const did = decodeURIComponent(params.did);
  const rkey = decodeURIComponent(params.rkey);
  const uri = `at://${did}/io.zzstoatzz.status.record/${rkey}`;

  try {
    const res = await callXrpc("dev.hatk.getRecord", { uri });
    if (res.record) {
      return { did, rkey, status: res.record };
    }
  } catch {}

  return { did, rkey, status: null };
};
