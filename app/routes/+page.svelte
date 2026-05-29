<script lang="ts">
  import { page } from '$app/stores'
  import { createQuery, useQueryClient } from '@tanstack/svelte-query'
  import { actorFeedQuery } from '$lib/queries'
  import { callXrpc } from '$hatk/client'
  import { isCustomEmoji, customEmojiName, bufoImageUrl, handleBufoError, parseLinks, parseStatusUri } from '$lib/utils/emoji'
  import { relativeTime, formatExpiration } from '$lib/utils/time'
  import LoginCard from '$lib/components/LoginCard.svelte'
  import CreateStatusForm from '$lib/components/CreateStatusForm.svelte'
  import StatusCard from '$lib/components/StatusCard.svelte'
  import { Link, Code, X } from 'lucide-svelte'

  const queryClient = useQueryClient()
  const viewer = $derived($page.data.viewer)

  const feed = createQuery(() => ({
    ...actorFeedQuery(viewer?.did ?? ''),
    enabled: !!viewer,
  }))

  const statuses = $derived((feed.data?.items ?? []) as any[])
  const current = $derived(statuses[0] ?? null)
  const history = $derived(statuses.slice(1))

  let copied = $state(false)
  let showEmbed = $state(false)

  function refresh() {
    queryClient.invalidateQueries({ queryKey: ['getFeed', 'actor'] })
  }

  async function deleteStatus(rkey: string) {
    if (!confirm('Delete this status?')) return
    try {
      await callXrpc('dev.hatk.deleteRecord', {
        collection: 'io.zzstoatzz.status.record',
        rkey,
      })
      refresh()
    } catch (err: any) {
      alert('Failed to delete: ' + (err?.message ?? err))
    }
  }

  async function shareStatus(uri: string) {
    const { did, rkey } = parseStatusUri(uri)
    const permalink = `${window.location.origin}/status/${did}/${rkey}`
    try {
      await navigator.clipboard.writeText(permalink)
      copied = true
      setTimeout(() => copied = false, 1500)
    } catch {}
  }
</script>

{#if !viewer}
  <LoginCard />
{:else}
  <div class="profile-card">
    <div class="current-status">
      {#if current}
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
            since {relativeTime(current.createdAt)}
            {#if current.expires}
              &middot; {formatExpiration(current.expires)}
            {/if}
          </span>
        </div>
        <div class="current-status-actions">
          <button class="share-btn" onclick={() => shareStatus(current.uri)} title="copy link">
            <Link size={16} />
          </button>
          <button class="embed-toggle-btn" onclick={() => showEmbed = !showEmbed} title="get embed code">
            <Code size={16} />
          </button>
          <button class="delete-btn" onclick={() => deleteStatus(parseStatusUri(current.uri).rkey)} title="delete">
            <X size={16} />
          </button>
        </div>
      {:else}
        <span class="big-emoji">-</span>
      {/if}
    </div>
  </div>

  <CreateStatusForm currentEmoji={current?.emoji ?? '😊'} oncreated={refresh} />

  {#if history.length > 0}
    <section class="history">
      <h2>history</h2>
      <div class="feed-list">
        {#each history as status (status.uri)}
          <StatusCard {status} showDelete ondelete={deleteStatus} />
        {/each}
      </div>
    </section>
  {/if}
{/if}
