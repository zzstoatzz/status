<script lang="ts">
  import { onMount } from 'svelte'
  import { callXrpc } from '$hatk/client'
  import {
    loadBufoList,
    searchBufos,
    loadEmojiData,
    searchEmojis,
    loadPopularEmoji,
    DEFAULT_FREQUENT,
  } from '$lib/utils/emoji'
  import CustomEmoji from './CustomEmoji.svelte'
  import GifImage from './GifImage.svelte'
  import { loadGifCatalog, searchGifs, type GifPost, type GifRef } from '$lib/utils/gifdex'

  let { onselect, onclose }: {
    // a gif still carries an emoji: it is the fallback for anything that cannot
    // render the gif, and what the popularity feed counts.
    onselect: (emoji: string, gif?: GifRef) => void
    onclose: () => void
  } = $props()

  type GridItem = {
    type: 'emoji' | 'bufo' | 'gif'
    value: string
    name?: string
    score?: number
    gif?: GifPost
  }

  // stands in for a gif wherever an emoji is required
  const GIF_FALLBACK_EMOJI = '\u{1F39E}\u{FE0F}'

  let currentCategory = $state('popular')
  let searchQuery = $state('')
  let gridItems: GridItem[] = $state([])
  let loading = $state(false)
  let bufoSearchTimer: ReturnType<typeof setTimeout> | undefined
  // Every render/search takes a ticket. An async result only gets to write the
  // grid if no newer one started meanwhile — comparing the query alone was not
  // enough, because switching tabs keeps the query and so let a still-pending
  // bufo search overwrite the tab you had just moved to.
  let generation = 0

  // Only this many tiles are in the DOM at once. The custom tab is ~1600 bufos;
  // rendering them all is what made opening it hang for seconds. The sentinel at
  // the end of the grid extends the window as it scrolls into view.
  const WINDOW_STEP = 60
  // Gifs are the real bytes — median ~1MB, no thumbnail tier — so they extend in
  // much smaller steps than emoji do. Raise this once a resizer exists.
  const GIF_WINDOW_STEP = 12
  // the ⭐ tab is a leaderboard, not a feed — one screenful of the all-time top
  const POPULAR_LIMIT = 64
  let visibleCount = $state(WINDOW_STEP)
  let sentinel: HTMLDivElement | undefined = $state()
  let gridEl: HTMLDivElement | undefined = $state()

  // names whose image has settled, so a tile knows to drop its skeleton
  let settled = $state(new Set<string>())

  let visibleItems = $derived(gridItems.slice(0, visibleCount))
  let hasMore = $derived(visibleCount < gridItems.length)
  // a search spans bufos too, so results can be mixed — size the cells for images
  // whenever any are present rather than keying off the tab alone.
  let isBufoGrid = $derived(
    currentCategory === 'custom' || gridItems.some((i) => i.type === 'bufo'),
  )
  let isGifGrid = $derived(currentCategory === 'gifs' || gridItems.some((i) => i.type === 'gif'))

  const categories = [
    { id: 'popular', icon: '⭐' },
    { id: 'custom', icon: '🐸' },
    { id: 'gifs', icon: '🎞️' },
    { id: 'people', icon: '😊' },
    { id: 'nature', icon: '🌿' },
    { id: 'food', icon: '🍔' },
    { id: 'activity', icon: '⚽' },
    { id: 'travel', icon: '✈️' },
    { id: 'objects', icon: '💡' },
    { id: 'symbols', icon: '💕' },
    { id: 'flags', icon: '🏁' },
  ]

  /** Reset the window and scroll whenever the result set changes identity. */
  let windowStep = $derived(currentCategory === 'gifs' ? GIF_WINDOW_STEP : WINDOW_STEP)

  function resetWindow() {
    visibleCount = windowStep
    gridEl?.scrollTo({ top: 0 })
  }

  async function renderCategory(cat: string) {
    currentCategory = cat
    // NOTE: searchQuery is deliberately NOT cleared. Switching tabs used to wipe
    // what you had typed, which is the one thing you never want a tab to do.
    if (searchQuery.trim()) {
      runSearch()
      return
    }

    const mine = ++generation
    loading = true
    resetWindow()

    let next: GridItem[]
    if (cat === 'custom') {
      next = (await loadBufoList().catch(() => [] as string[])).map((name) => ({
        type: 'bufo' as const,
        value: `custom:${name}`,
        name,
      }))
    } else if (cat === 'gifs') {
      const gifs = await loadGifCatalog().catch(() => [] as GifPost[])
      next = gifs.map((g) => ({ type: 'gif' as const, value: g.uri, name: g.title, gif: g }))
    } else if (cat === 'popular') {
      const popular = await loadPopularEmoji(() =>
        callXrpc('dev.hatk.getFeed', { feed: 'popular', limit: POPULAR_LIMIT }),
      ).catch(() => [...DEFAULT_FREQUENT])
      next = popular.map((value) =>
        value.startsWith('custom:')
          ? { type: 'bufo' as const, value, name: value.slice(7) }
          : { type: 'emoji' as const, value },
      )
    } else {
      const data = await loadEmojiData().catch(() => null)
      next = (data?.categories[cat] ?? []).map((e) => ({ type: 'emoji' as const, value: e }))
    }

    if (mine !== generation) return
    gridItems = next
    loading = false
  }

  /** The active tab scopes the query: the bufo tab searches bufos, everything
      else searches unicode and appends name-matching bufos. */
  async function runSearch() {
    const q = searchQuery.trim()
    if (!q) {
      renderCategory(currentCategory)
      return
    }

    const mine = ++generation
    resetWindow()

    if (currentCategory === 'gifs') {
      // the whole catalog is already local, so this filters without a network hop
      const gifs = await loadGifCatalog().catch(() => [] as GifPost[])
      if (mine !== generation) return
      gridItems = searchGifs(q, gifs).map((g) => ({
        type: 'gif' as const,
        value: g.uri,
        name: g.title,
        gif: g,
      }))
      loading = false
      return
    }

    if (currentCategory === 'custom') {
      // semantic search is a network call per keystroke; debounce it
      clearTimeout(bufoSearchTimer)
      loading = true
      bufoSearchTimer = setTimeout(async () => {
        const results = await searchBufos(q, 60).catch(() => [])
        if (mine !== generation) return
        gridItems = results.map((r) => ({
          type: 'bufo' as const,
          value: `custom:${r.name}`,
          name: r.name,
          score: r.score,
        }))
        loading = false
      }, 250)
      return
    }

    loading = true
    const data = await loadEmojiData().catch(() => null)
    const bufos = await loadBufoList().catch(() => [] as string[])
    if (mine !== generation) return

    const qLower = q.toLowerCase()
    gridItems = [
      ...(data ? searchEmojis(q, data) : []).map((e) => ({ type: 'emoji' as const, value: e })),
      ...bufos
        .filter((name) => name.toLowerCase().includes(qLower))
        .slice(0, 60)
        .map((name) => ({ type: 'bufo' as const, value: `custom:${name}`, name })),
    ]
    loading = false
  }

  function select(value: string, gif?: GifPost) {
    onselect(gif ? GIF_FALLBACK_EMOJI : value, gif ? { uri: gif.uri, cid: gif.cid } : undefined)
    onclose()
  }

  onMount(() => {
    // lock the page behind the sheet: without this, scroll chaining and the mobile
    // keyboard resizing the visual viewport yank the underlying page around.
    const { overflow, paddingRight } = document.body.style
    const gutter = window.innerWidth - document.documentElement.clientWidth
    document.body.style.overflow = 'hidden'
    if (gutter > 0) document.body.style.paddingRight = `${gutter}px`

    renderCategory('popular')

    // warm the two lists the picker always ends up wanting, off the critical path
    loadEmojiData().catch(() => {})
    loadBufoList().catch(() => {})

    return () => {
      document.body.style.overflow = overflow
      document.body.style.paddingRight = paddingRight
      clearTimeout(bufoSearchTimer)
    }
  })

  // extend the window slightly before the sentinel is actually on screen, so the
  // next rows are already decoding by the time they are scrolled to.
  $effect(() => {
    if (!sentinel || !gridEl) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting) && visibleCount < gridItems.length) {
          visibleCount = Math.min(visibleCount + windowStep, gridItems.length)
        }
      },
      { root: gridEl, rootMargin: '300px' },
    )
    observer.observe(sentinel)
    return () => observer.disconnect()
  })
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onclose() }} />

<div class="emoji-picker-overlay" role="presentation" onpointerdown={(e) => { if (e.target === e.currentTarget) onclose() }}>
  <div class="emoji-picker" role="dialog" aria-modal="true" aria-label="pick an emoji">
    <div class="emoji-picker-header">
      <span class="emoji-picker-grabber" aria-hidden="true"></span>
      <h3>pick an emoji</h3>
      <button class="emoji-picker-close" onclick={onclose} aria-label="close emoji picker">&#x2715;</button>
    </div>
    <input
        type="search"
        class="emoji-search"
        placeholder={currentCategory === 'custom'
          ? 'describe a bufo… "happy", "apocalyptic"'
          : currentCategory === 'gifs'
            ? 'search gifs by title or tag…'
            : 'search emojis…'}
        autocomplete="off"
        autocapitalize="off"
        autocorrect="off"
        spellcheck="false"
        enterkeyhint="search"
        bind:value={searchQuery}
        oninput={runSearch}
        onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); (e.currentTarget as HTMLInputElement).blur() } }}
    />
    <div class="emoji-categories" role="tablist" aria-label="emoji categories">
      {#each categories as cat (cat.id)}
        <button
          class="category-btn"
          class:active={currentCategory === cat.id}
          role="tab"
          aria-selected={currentCategory === cat.id}
          aria-label={cat.id}
          title={cat.id}
          onclick={() => renderCategory(cat.id)}
        >{cat.icon}</button>
      {/each}
    </div>
    <div class="emoji-grid-wrap">
      <div class="emoji-loading-bar" class:visible={loading} aria-hidden="true"></div>
      <div class="emoji-grid" class:bufo-grid={isBufoGrid} class:gif-grid={isGifGrid} bind:this={gridEl}>
        {#if !loading && gridItems.length === 0}
          <div class="no-results">no emojis found</div>
        {:else}
          {#each visibleItems as item (item.value)}
            {#if item.type === 'gif' && item.gif}
              <button
                class="emoji-btn gif-btn"
                class:pending={!settled.has(item.value)}
                onclick={() => select(item.value, item.gif)}
                title={item.gif.title ?? 'gif'}
                style={item.gif.width && item.gif.height
                  ? `aspect-ratio: ${item.gif.width} / ${item.gif.height}`
                  : undefined}
              >
                <GifImage
                  did={item.gif.did}
                  blobCid={item.gif.blobCid}
                  alt={item.gif.title ?? ''}
                  onsettled={() => { settled = new Set(settled).add(item.value) }}
                />
              </button>
            {:else if item.type === 'bufo'}
              <button
                class="emoji-btn bufo-btn"
                class:pending={!settled.has(item.value)}
                onclick={() => select(item.value)}
                title={item.name}
              >
                <CustomEmoji
                  name={item.name ?? ''}
                  loading="lazy"
                  onsettled={() => { settled = new Set(settled).add(item.value) }}
                />
                {#if item.score != null}
                  <span class="bufo-score">{Math.round(item.score * 100)}%</span>
                {/if}
              </button>
            {:else}
              <button class="emoji-btn" onclick={() => select(item.value)}>{item.value}</button>
            {/if}
          {/each}
          {#if hasMore}
            <div class="emoji-sentinel" bind:this={sentinel} aria-hidden="true"></div>
          {/if}
        {/if}
      </div>
    </div>
    <div class="bufo-helper">
      {#if currentCategory === 'custom'}
        <a href="https://find-bufo.com" target="_blank" rel="noopener">powered by find-bufo.com</a>
      {:else if currentCategory === 'gifs'}
        <span>gifs from net.gifdex.gif.post on atproto</span>
      {/if}
    </div>
  </div>
</div>
