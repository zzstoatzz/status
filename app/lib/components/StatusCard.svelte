<script lang="ts">
  import { isCustomEmoji, customEmojiName, bufoImageUrl, bufoFallbackUrl, parseLinks, parseStatusUri } from '$lib/utils/emoji'
  import { relativeTime, formatExpiration } from '$lib/utils/time'
  import { Link, X } from 'lucide-svelte'

  interface StatusItem {
    uri: string
    emoji: string
    text?: string
    handle?: string
    did?: string
    createdAt: string
    expires?: string
    expired?: boolean
  }

  let {
    status,
    showAuthor = false,
    showDelete = false,
    ondelete,
  }: {
    status: StatusItem
    showAuthor?: boolean
    showDelete?: boolean
    ondelete?: (rkey: string) => void
  } = $props()

  let copied = $state(false)

  function getPermalink() {
    const { did, rkey } = parseStatusUri(status.uri)
    return `${window.location.origin}/status/${did}/${rkey}`
  }

  async function share() {
    try {
      await navigator.clipboard.writeText(getPermalink())
      copied = true
      setTimeout(() => copied = false, 1500)
    } catch {}
  }

  function handleDelete() {
    const { rkey } = parseStatusUri(status.uri)
    ondelete?.(rkey)
  }
</script>

<div class="status-item">
  <span class="emoji">
    {#if isCustomEmoji(status.emoji)}
      {@const name = customEmojiName(status.emoji)}
      <img src={bufoImageUrl(name)} alt={name} onerror={(e) => { e.currentTarget.src = bufoFallbackUrl(name) }} />
    {:else}
      {status.emoji}
    {/if}
  </span>
  <div class="content">
    <div>
      {#if showAuthor && (status.handle || status.did)}
        <a href="/@{status.handle ?? status.did}" class="author">@{status.handle ?? status.did?.slice(0, 18)}</a>
      {/if}
      {#if status.text}
        <span class="text">{@html parseLinks(status.text)}</span>
      {/if}
    </div>
    <span class="time">
      {relativeTime(status.createdAt)}
      {#if status.expires}
        &middot; {formatExpiration(status.expires)}
      {/if}
    </span>
  </div>
  <div class="status-actions">
    <button class="share-btn" class:copied onclick={share} title="copy link">
      <Link size={14} />
    </button>
    {#if showDelete && ondelete}
      <button class="delete-btn" onclick={handleDelete} title="delete">
        <X size={14} />
      </button>
    {/if}
  </div>
</div>
