<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query'
  import { recentFeedQuery } from '$lib/queries'
  import StatusFeed from '$lib/components/StatusFeed.svelte'

  const feed = createQuery(() => recentFeedQuery())
</script>

<svelte:head>
  <title>global feed — status</title>
</svelte:head>

{#if feed.isLoading}
  <div class="center">loading...</div>
{:else}
  <StatusFeed
    feed="recent"
    initialItems={feed.data?.items ?? []}
    initialCursor={feed.data?.cursor}
    showAuthor
  />
{/if}
