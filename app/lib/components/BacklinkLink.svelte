<script lang="ts">
  import { atprotoClient, postLink, resolveClient } from '$lib/atclients'
  import { fetchBacklink, resolveHandles, type Backlink } from '$lib/utils/backlinks'

  let { permalink }: { permalink: string } = $props()

  let backlink = $state<Backlink | null>(null)
  let el: HTMLSpanElement | undefined = $state()
  let open = $state(false)
  let handles: Record<string, string> = $state({})

  let client = $derived(resolveClient($atprotoClient))
  let posts = $derived(backlink?.posts ?? [])
  let many = $derived(posts.length > 1)

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

  /** Names for the menu, fetched once when it is first opened — never for a
      status with a single backlink, which needs no menu at all. */
  async function toggle() {
    open = !open
    if (!open || Object.keys(handles).length > 0) return
    handles = await resolveHandles(posts.map((p) => p.did))
  }

  function label(did: string): string {
    return handles[did] ? `@${handles[did]}` : `${did.slice(0, 16)}…`
  }
</script>

<svelte:window
  onclick={(e) => {
    if (open && el && !el.contains(e.target as Node)) open = false
  }}
  onkeydown={(e) => { if (e.key === 'Escape') open = false }}
/>

<span bind:this={el} class="backlink-slot">
  {#if many}
    <button
      class="backlink-btn"
      onclick={toggle}
      aria-expanded={open}
      aria-haspopup="menu"
      title={`referenced by ${backlink?.count} bluesky posts`}
    >
      <img src={client.iconUrl} alt="" width="14" height="14" loading="lazy" />
      <span class="backlink-count">{backlink?.count}</span>
    </button>
    {#if open}
      <div class="backlink-menu" role="menu">
        {#each posts as p (p.did + p.rkey)}
          <a
            role="menuitem"
            href={postLink($atprotoClient, p.did, p.rkey)}
            target="_blank"
            rel="noopener"
            onclick={() => (open = false)}
          >
            {label(p.did)}
          </a>
        {/each}
      </div>
    {/if}
  {:else if posts.length === 1}
    <a
      class="backlink-btn"
      href={postLink($atprotoClient, posts[0].did, posts[0].rkey)}
      target="_blank"
      rel="noopener"
      title={`referenced by a bluesky post — open in ${client.label}`}
    >
      <!-- the selected client's own mark; every one in the registry is
           transparent-background, so it sits on the card without a plate -->
      <img src={client.iconUrl} alt="" width="14" height="14" loading="lazy" />
    </a>
  {/if}
</span>
