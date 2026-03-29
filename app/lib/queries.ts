import { queryOptions } from "@tanstack/svelte-query";
import { callXrpc } from "$hatk/client";

type Fetch = typeof fetch;

export const recentFeedQuery = (limit = 50, f?: Fetch) =>
  queryOptions({
    queryKey: ["getFeed", "recent"],
    queryFn: () => callXrpc("dev.hatk.getFeed", { feed: "recent", limit }, f),
    staleTime: 60_000,
  });

export const actorFeedQuery = (did: string, limit = 50, f?: Fetch) =>
  queryOptions({
    queryKey: ["getFeed", "actor", did],
    queryFn: () =>
      callXrpc("dev.hatk.getFeed", { feed: "actor", actor: did, limit }, f),
    staleTime: 60_000,
  });
