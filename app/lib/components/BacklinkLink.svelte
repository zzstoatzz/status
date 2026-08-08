<script lang="ts">
  import { atprotoClient, postLink, resolveClient } from '$lib/atclients'
  import { fetchBacklink, type Backlink } from '$lib/utils/backlinks'

  let { permalink }: { permalink: string } = $props()

  let backlink: Backlink | null = $state(null)
  let el: HTMLSpanElement | undefined = $state()

  let client = $derived(resolveClient($atprotoClient))

  // one request per status, and only once it is actually on screen — a long
  // feed should not fire a burst at constellation for rows nobody scrolls to
  $effect(() => {
    if (!el || !permalink) return
    // observe the surrounding card, not this span: until a backlink resolves the
    // span is empty, and a zero-area target is not reliably reported as visible
    const target = el.closest('.status-item, .current-status') ?? el
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return
      observer.disconnect()
      fetchBacklink(permalink).then((r) => { backlink = r })
    })
    observer.observe(target)
    return () => observer.disconnect()
  })
</script>

<span bind:this={el} class="backlink-slot">
  {#if backlink}
    <a
      class="backlink-btn"
      href={postLink($atprotoClient, backlink.did, backlink.rkey)}
      target="_blank"
      rel="noopener"
      title={backlink.count === 1
        ? `referenced by a bluesky post — open in ${client.label}`
        : `referenced by ${backlink.count} bluesky posts — open the latest in ${client.label}`}
    >
      <!-- the selected client's own mark; every one in the registry is
           transparent-background, so it sits on the card without a plate -->
      <img src={client.iconUrl} alt="" width="14" height="14" loading="lazy" />
      {#if backlink.count > 1}<span class="backlink-count">{backlink.count}</span>{/if}
    </a>
  {/if}
</span>
