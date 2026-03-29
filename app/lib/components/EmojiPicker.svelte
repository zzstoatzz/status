<script lang="ts">
  import { loadBufoList, searchBufos, loadEmojiData, searchEmojis, DEFAULT_FREQUENT } from '$lib/utils/emoji'

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

  $effect(() => {
    renderCategory('frequent')
  })
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="emoji-picker-overlay" onclick={(e) => { if (e.target === e.currentTarget) onclose() }}>
  <div class="emoji-picker">
    <div class="emoji-picker-header">
      <h3>pick an emoji</h3>
      <button class="emoji-picker-close" onclick={onclose}>&#x2715;</button>
    </div>
    <input
      type="text"
      class="emoji-search"
      placeholder={currentCategory === 'custom' ? 'describe a bufo... try "happy" or "apocalyptic"' : 'search emojis...'}
      bind:value={searchQuery}
      oninput={handleSearch}
    />
    <div class="emoji-categories">
      {#each categories as cat (cat.id)}
        <button
          class="category-btn"
          class:active={currentCategory === cat.id}
          onclick={() => renderCategory(cat.id)}
        >{cat.icon}</button>
      {/each}
    </div>
    <div class="emoji-grid" class:bufo-grid={currentCategory === 'custom' || gridItems.some(i => i.type === 'bufo')}>
      {#if loading}
        <div class="loading">loading...</div>
      {:else if gridItems.length === 0}
        <div class="no-results">no emojis found</div>
      {:else}
        {#each gridItems as item (item.value)}
          {#if item.type === 'bufo'}
            <button class="emoji-btn bufo-btn" onclick={() => select(item.value)} title={item.name}>
              <img src="https://all-the.bufo.zone/{item.name}.png" alt={item.name ?? ''} loading="lazy" onerror={(e) => { const img = e.currentTarget as HTMLImageElement; img.src = img.src.replace('.png', '.gif') }} />
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
    {#if currentCategory === 'custom'}
      <div class="bufo-helper">
        <a href="https://find-bufo.com" target="_blank" rel="noopener">powered by find-bufo.com</a>
      </div>
    {/if}
  </div>
</div>
