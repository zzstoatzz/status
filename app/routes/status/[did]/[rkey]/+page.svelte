<script lang="ts">
  import { isCustomEmoji, customEmojiName, bufoImageUrl, handleBufoError, parseLinks } from '$lib/utils/emoji'
  import { relativeTime, formatExpiration } from '$lib/utils/time'

  let { data } = $props()

  let status = $derived(data.status)
  let emoji = $derived(status?.emoji ?? status?.value?.emoji)
  let text = $derived(status?.text ?? status?.value?.text)
  let handle = $derived(status?.handle ?? status?.value?.handle ?? data.did)
  let createdAt = $derived(status?.createdAt ?? status?.value?.createdAt)
  let expires = $derived(status?.expires ?? status?.value?.expires)

  let ogTitle = $derived(`@${handle}'s status`)
  let ogDescription = $derived(text || (emoji && isCustomEmoji(emoji) ? customEmojiName(emoji).replace(/-/g, ' ') : emoji) || 'share your status')
  let ogUrl = $derived(`https://status.zzstoatzz.io/status/${data.did}/${data.rkey}`)
  let ogImage = $derived(data.ogImage ?? null)
</script>

<svelte:head>
  <title>{status ? ogTitle : 'status not found'} | status</title>
  {#if status}
    <meta property="og:type" content="website" />
    <meta property="og:title" content={ogTitle} />
    <meta property="og:description" content={ogDescription} />
    <meta property="og:url" content={ogUrl} />
    <meta property="og:site_name" content="status" />
    {#if ogImage}
      <meta property="og:image" content={ogImage} />
      <meta name="twitter:image" content={ogImage} />
      <meta name="twitter:card" content="summary_large_image" />
    {:else}
      <meta name="twitter:card" content="summary" />
    {/if}
    <meta name="twitter:title" content={ogTitle} />
    <meta name="twitter:description" content={ogDescription} />
  {/if}
</svelte:head>

{#if status}
  <div class="profile-card">
    <div class="current-status">
      <span class="big-emoji">
        {#if emoji && isCustomEmoji(emoji)}
          {@const name = customEmojiName(emoji)}
          <img src={bufoImageUrl(name)} alt={name} title={name} onerror={(e) => handleBufoError(e.currentTarget as HTMLImageElement, name)} />
        {:else}
          {emoji ?? '-'}
        {/if}
      </span>
      <div class="status-info">
        {#if text}
          <span class="current-text">{@html parseLinks(text)}</span>
        {/if}
        <span class="meta">
          {#if createdAt}{relativeTime(createdAt)}{/if}
          {#if expires}
            &middot; {formatExpiration(expires)}
          {/if}
        </span>
      </div>
    </div>
  </div>
  <div class="center">
    <a href="/@{handle}" class="view-profile-link">view all statuses</a>
  </div>
{:else}
  <div class="center">status not found</div>
{/if}
