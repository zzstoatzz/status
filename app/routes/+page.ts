import { browser } from "$app/environment";
import { actorFeedQuery } from "$lib/queries";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ parent }) => {
  const { queryClient, viewer } = await parent();
  if (viewer) {
    const prefetch = queryClient.prefetchQuery(actorFeedQuery(viewer.did));
    if (!browser) await prefetch;
  }
};
