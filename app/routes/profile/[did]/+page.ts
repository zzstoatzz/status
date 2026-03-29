import { browser } from "$app/environment";
import { actorFeedQuery } from "$lib/queries";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ params, parent, fetch }) => {
  const did = decodeURIComponent(params.did);
  const { queryClient } = await parent();
  const prefetch = queryClient.prefetchQuery(actorFeedQuery(did, 50, fetch));
  if (!browser) await prefetch;
  return { did };
};
