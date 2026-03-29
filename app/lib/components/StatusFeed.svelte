<script lang="ts">
  import { untrack } from 'svelte'
  import { callXrpc } from '$hatk/client'
  import StatusCard from './StatusCard.svelte'

  interface StatusItem {
    uri: string
    cid: string
    did: string
    handle: string
    emoji: string
    text?: string
    expires?: string
    createdAt: string
    indexedAt: string
    expired: boolean
  }

  let {
    feed,
    initialItems = [],
    initialCursor,
    showAuthor = false,
    showDelete = false,
    ondelete,
  }: {
    feed: string
    initialItems?: StatusItem[]
    initialCursor?: string
    showAuthor?: boolean
    showDelete?: boolean
    ondelete?: (rkey: string) => void
  } = $props()

  let items: StatusItem[] = $state(untrack(() => [...initialItems]))
  let cursor: string | undefined = $state(untrack(() => initialCursor))
  let loadingMore = $state(false)
  let hasMore = $derived(!!cursor)

  async function loadMore() {
    if (!cursor || loadingMore) return
    loadingMore = true
    try {
      const res = await callXrpc('dev.hatk.getFeed', { feed, cursor, limit: 20 })
      items = [...items, ...(res.items ?? [])]
      cursor = res.cursor
    } catch (err) {
      console.error('Failed to load more:', err)
    } finally {
      loadingMore = false
    }
  }
</script>

<div class="feed-list">
  {#each items as status (status.uri)}
    <StatusCard {status} {showAuthor} {showDelete} {ondelete} />
  {/each}
</div>

{#if hasMore}
  <div class="load-more">
    <button onclick={loadMore} disabled={loadingMore}>
      {loadingMore ? 'loading...' : 'load more'}
    </button>
  </div>
{:else if items.length > 0}
  <div class="end-of-feed">you've reached the end</div>
{/if}
