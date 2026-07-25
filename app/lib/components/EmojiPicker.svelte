<script lang="ts">
  import { onMount } from 'svelte'
  import { loadBufoList, searchBufos, loadEmojiData, searchEmojis, DEFAULT_FREQUENT } from '$lib/utils/emoji'
  import CustomEmoji from './CustomEmoji.svelte'

  let { onselect, onclose }: { onselect: (emoji: string) => void; onclose: () => void } = $props()

  let currentCategory = $state('frequent')
  let searchQuery = $state('')
  let gridItems: Array<{ type: 'emoji' | 'bufo'; value: string; name?: string; score?: number }> = $state([])
  let loading = $state(false)
  let bufoSearchTimer: ReturnType<typeof setTimeout> | undefined

  const categories = [
    { id: 'frequent', icon: '\u2B50' },
    { id: 'custom', icon: '\uD83D\uDC38' },
    { id: 'people', icon: '\uD83D\uDE0A' },
    { id: 'nature', icon: '\uD83C\uDF3F' },
    { id: 'food', icon: '\uD83C\uDF54' },
    { id: 'activity', icon: '\u26BD' },
    { id: 'travel', icon: '\u2708\uFE0F' },
    { id: 'objects', icon: '\uD83D\uDCA1' },
    { id: 'symbols', icon: '\uD83D\uDC95' },
    { id: 'flags', icon: '\uD83C\uDFC1' },
  ]

  async function renderCategory(cat: string) {
    currentCategory = cat
    searchQuery = ''
    loading = true

    if (cat === 'custom') {
      try {
        const bufos = await loadBufoList()
        gridItems = bufos.map(name => ({ type: 'bufo', value: `custom:${name}`, name }))
      } catch {
        gridItems = []
      }
    } else if (cat === 'frequent') {
      gridItems = DEFAULT_FREQUENT.map(e => ({ type: 'emoji', value: e }))
    } else {
      try {
        const data = await loadEmojiData()
        const emojis = data.categories[cat] || []
        gridItems = emojis.map(e => ({ type: 'emoji', value: e }))
      } catch {
        gridItems = []
      }
    }
    loading = false
  }

  async function handleSearch() {
    const q = searchQuery.trim()
    if (!q) {
      renderCategory(currentCategory)
      return
    }

    if (currentCategory === 'custom') {
      clearTimeout(bufoSearchTimer)
      bufoSearchTimer = setTimeout(async () => {
        loading = true
        try {
          const results = await searchBufos(q, 30)
          if (searchQuery.trim() !== q) return
          gridItems = results.map(r => ({ type: 'bufo', value: `custom:${r.name}`, name: r.name, score: r.score }))
        } catch {
          gridItems = []
        }
        loading = false
      }, 300)
      return
    }

    loading = true
    try {
      const data = await loadEmojiData()
      const emojiResults = searchEmojis(q, data)
      const bufos = await loadBufoList().catch(() => [] as string[])
      const qLower = q.toLowerCase()
      const bufoResults = bufos.filter(name => name.toLowerCase().includes(qLower)).slice(0, 30)

      gridItems = [
        ...emojiResults.map(e => ({ type: 'emoji' as const, value: e })),
        ...bufoResults.map(name => ({ type: 'bufo' as const, value: `custom:${name}`, name })),
      ]
    } catch {
      gridItems = []
    }
    loading = false
  }

  function select(value: string) {
    onselect(value)
    onclose()
  }

  // lock the page behind the sheet: without this, scroll chaining and the mobile
  // keyboard resizing the visual viewport yank the underlying page around.
  onMount(() => {
    const { overflow, paddingRight } = document.body.style
    const gutter = window.innerWidth - document.documentElement.clientWidth
    document.body.style.overflow = 'hidden'
    if (gutter > 0) document.body.style.paddingRight = `${gutter}px`

    renderCategory('frequent')

    return () => {
      document.body.style.overflow = overflow
      document.body.style.paddingRight = paddingRight
    }
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
      placeholder={currentCategory === 'custom' ? 'describe a bufo… "happy", "apocalyptic"' : 'search emojis…'}
      autocomplete="off"
      autocapitalize="off"
      autocorrect="off"
      spellcheck="false"
      enterkeyhint="search"
      bind:value={searchQuery}
      oninput={handleSearch}
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
      <div class="emoji-grid" class:bufo-grid={currentCategory === 'custom' || gridItems.some(i => i.type === 'bufo')}>
      {#if !loading && gridItems.length === 0}
        <div class="no-results">no emojis found</div>
      {:else}
        {#each gridItems as item (item.value)}
          {#if item.type === 'bufo'}
            <button class="emoji-btn bufo-btn" onclick={() => select(item.value)} title={item.name}>
              <CustomEmoji name={item.name ?? ''} loading="lazy" />
              {#if item.score != null}
                <span class="bufo-score">{Math.round(item.score * 100)}%</span>
              {/if}
            </button>
          {:else}
            <button class="emoji-btn" onclick={() => select(item.value)}>{item.value}</button>
          {/if}
        {/each}
      {/if}
      </div>
    </div>
    <div class="bufo-helper">
      {#if currentCategory === 'custom'}
        <a href="https://find-bufo.com" target="_blank" rel="noopener">powered by find-bufo.com</a>
      {/if}
    </div>
  </div>
</div>
