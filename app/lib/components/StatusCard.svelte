<script lang="ts">
  import { isCustomEmoji, customEmojiName, parseLinks, parseStatusUri } from '$lib/utils/emoji'
  import CustomEmoji from './CustomEmoji.svelte'
  import GifImage from './GifImage.svelte'
  import { gifFromRef, fetchGifTitle, type GifRef } from '$lib/utils/gifdex'
  import { callXrpc } from '$hatk/client'
  import { toast } from '$lib/toast.svelte'
  import { relativeTime, formatExpiration, absoluteTime } from '$lib/utils/time'
  import { Link, X } from 'lucide-svelte'
  import { browser } from '$app/environment'
  import BacklinkLink from './BacklinkLink.svelte'
  import { statusPermalink } from '$lib/utils/backlinks'

  interface StatusItem {
    uri: string
    emoji: string
    gif?: GifRef | string
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


  // the same string the copy button yields — constellation targets are exact,
  // so a backlink lookup has to use the permalink people actually shared
  let permalink = $derived(browser ? statusPermalink(window.location.origin, status.uri) : '')

  // The status stores only a strongRef, so the gif's name comes from our own
  // index. Native tooltips must be set before the pointer arrives, so this
  // cannot wait for hover.
  let hasAuthor = $derived(showAuthor && !!(status.handle || status.did))

  let gifTitle = $state<string | null>(null)
  $effect(() => {
    const uri = typeof status.gif === 'string' ? status.gif : status.gif?.uri
    if (!browser || !uri || !gifFromRef(status.gif)) return
    fetchGifTitle(
      (nsid, params) => callXrpc(nsid as Parameters<typeof callXrpc>[0], params as never),
      uri,
    ).then((t) => { gifTitle = t })
  })

  async function share() {
    try {
      await navigator.clipboard.writeText(permalink)
      toast.success('link copied')
    } catch {
      toast.error('could not copy link')
    }
  }

  function handleDelete() {
    const { rkey } = parseStatusUri(status.uri)
    ondelete?.(rkey)
  }
</script>

<div class="status-item">
  <span class="emoji">
    {#if gifFromRef(status.gif)}
      {@const g = gifFromRef(status.gif)!}
      <GifImage
        did={g.did}
        blobCid={g.blobCid}
        source={g.source}
        alt={gifTitle ?? status.text ?? 'gif status'}
        title={gifTitle ?? undefined}
      />
    {:else if isCustomEmoji(status.emoji)}
      {@const name = customEmojiName(status.emoji)}
      <CustomEmoji {name} loading="lazy" />
    {:else}
      {status.emoji}
    {/if}
  </span>
  <!-- two rows: who, then what. when lives in the trailing column. -->
  <div class="content" class:with-author={hasAuthor}>
    {#if hasAuthor}
      <a href="/@{status.handle ?? status.did}" class="author">@{status.handle ?? status.did?.slice(0, 18)}</a>
    {/if}
    {#if status.text}
      <span class="text" title={status.text}>{@html parseLinks(status.text)}</span>
    {/if}
  </div>
  <!-- one trailing slot: when it happened, until you reach for what to do about
       it. Gmail's swap — the two never compete for the same space. -->
  <div class="trailing">
    <span class="time">
      <time datetime={status.createdAt} title={absoluteTime(status.createdAt)}>
        {relativeTime(status.createdAt)}
      </time>{#if status.expires}<span class="expiry"
        >&middot; <time datetime={status.expires} title={absoluteTime(status.expires)}
          >{formatExpiration(status.expires)}</time
        ></span
      >{/if}
    </span>
    <div class="status-actions">
      {#if permalink}
        <BacklinkLink {permalink} />
      {/if}
      <button class="share-btn" onclick={share} title="copy link">
        <Link size={14} />
      </button>
      {#if showDelete && ondelete}
        <button class="delete-btn" onclick={handleDelete} title="delete">
          <X size={14} />
        </button>
      {/if}
    </div>
  </div>
</div>
