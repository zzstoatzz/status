<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query'
  import { actorFeedQuery } from '$lib/queries'
  import StatusFeed from '$lib/components/StatusFeed.svelte'
  import { isCustomEmoji, customEmojiName, bufoImageUrl, handleBufoError, parseLinks } from '$lib/utils/emoji'
  import { relativeTime, formatExpiration } from '$lib/utils/time'

  let { data } = $props()

  const feed = createQuery(() => actorFeedQuery(data.did))
  const statuses = $derived((feed.data?.items ?? []) as any[])
  const current = $derived(statuses[0] ?? null)
  const handle = $derived(current?.handle ?? data.did.slice(0, 18))
</script>

<svelte:head>
  <title>@{handle} — status</title>
</svelte:head>

{#if feed.isLoading}
  <div class="center">loading...</div>
{:else if !current}
  <div class="center">no statuses yet</div>
{:else}
  <div class="profile-card">
    <div class="current-status">
      <span class="big-emoji">
        {#if isCustomEmoji(current.emoji)}
          {@const name = customEmojiName(current.emoji)}
          <img src={bufoImageUrl(name)} alt={name} title={name} onerror={(e) => handleBufoError(e.currentTarget as HTMLImageElement, name)} />
        {:else}
          {current.emoji}
        {/if}
      </span>
      <div class="status-info">
        {#if current.text}
          <span class="current-text">{@html parseLinks(current.text)}</span>
        {/if}
        <span class="meta">
          {relativeTime(current.createdAt)}
          {#if current.expires}
            &middot; {formatExpiration(current.expires)}
          {/if}
        </span>
      </div>
    </div>
  </div>

  {#if statuses.length > 1}
    <section class="history">
      <h2>history</h2>
      <StatusFeed
        feed="actor"
        initialItems={statuses.slice(1)}
        showAuthor={false}
      />
    </section>
  {/if}
{/if}
