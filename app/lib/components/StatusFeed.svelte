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
    actor,
    initialItems = [],
    initialCursor,
    showAuthor = false,
    showDelete = false,
    ondelete,
  }: {
    feed: string
    /** required by the actor feed — without it the server returns nothing */
    actor?: string
    initialItems?: StatusItem[]
    initialCursor?: string
    showAuthor?: boolean
    showDelete?: boolean
    ondelete?: (rkey: string) => void
  } = $props()

  let items: StatusItem[] = $state(untrack(() => [...initialItems]))
  let cursor: string | undefined = $state(untrack(() => initialCursor))

  // Resync when the query behind us refetches — posting or deleting a status
  // has to show up. Writes only; reading `items` here would make this re-run
  // itself, which is the cycle that froze the emoji picker.
  $effect(() => {
    items = [...initialItems]
    cursor = initialCursor
  })
  let loadingMore = $state(false)
  let hasMore = $derived(!!cursor)

  async function loadMore() {
    if (!cursor || loadingMore) return
    loadingMore = true
    try {
      // `actor` matters: the actor feed returns an empty page without it, so
      // paging a profile or your own history silently yielded nothing
      const res = await callXrpc('dev.hatk.getFeed', { feed, actor, cursor, limit: 20 })
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
