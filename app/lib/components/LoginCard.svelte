<script lang="ts">
  import { login } from '$lib/auth'
  import { ChevronDown } from 'lucide-svelte'

  let handle = $state('')
  let suggestions: Array<{ handle: string; displayName?: string; avatar?: string }> = $state([])
  let selectedIndex = $state(-1)
  let showDropdown = $state(false)
  let debounceTimer: ReturnType<typeof setTimeout> | undefined
  let abortController: AbortController | null = null
  let faqOpen: Record<string, boolean> = $state({})

  async function fetchSuggestions(query: string) {
    if (abortController) abortController.abort()
    abortController = new AbortController()
    try {
      const url = `https://typeahead.waow.tech/xrpc/app.bsky.actor.searchActorsTypeahead?q=${encodeURIComponent(query)}&limit=5`
      const res = await fetch(url, { signal: abortController.signal })
      if (!res.ok) return []
      const data = await res.json()
      return data.actors || []
    } catch {
      return []
    }
  }

  function oninput() {
    const q = handle.trim()
    clearTimeout(debounceTimer)
    if (q.length < 3) {
      suggestions = []
      showDropdown = false
      return
    }
    debounceTimer = setTimeout(async () => {
      suggestions = await fetchSuggestions(q)
      selectedIndex = -1
      showDropdown = suggestions.length > 0
    }, 300)
  }

  function selectSuggestion(h: string) {
    handle = h
    showDropdown = false
    suggestions = []
  }

  function onkeydown(e: KeyboardEvent) {
    if (!showDropdown || suggestions.length === 0) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      selectedIndex = Math.min(selectedIndex + 1, suggestions.length - 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      selectedIndex = Math.max(selectedIndex - 1, -1)
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      e.preventDefault()
      selectSuggestion(suggestions[selectedIndex].handle)
    } else if (e.key === 'Escape') {
      showDropdown = false
    }
  }

  async function submit(e: Event) {
    e.preventDefault()
    const h = handle.trim()
    if (h) await login(h)
  }

  function toggleFaq(id: string) {
    faqOpen[id] = !faqOpen[id]
  }
</script>

<div class="login-container">
  <div class="login-card">
    <h2 class="login-title">what's happening?</h2>
    <p class="login-tagline">share what you're up to</p>
    <form class="login-form" onsubmit={submit}>
      <div class="input-group">
        <label for="handle-input">internet handle</label>
        <div class="handle-input-wrapper">
          <input
            id="handle-input"
            type="text"
            placeholder="you.bsky.social"
            autocomplete="off"
            spellcheck="false"
            required
            bind:value={handle}
            {oninput}
            {onkeydown}
            onblur={() => setTimeout(() => showDropdown = false, 200)}
            onfocus={() => { if (handle.trim().length >= 3 && suggestions.length > 0) showDropdown = true }}
          />
          {#if showDropdown}
            <div class="suggestions-dropdown">
              {#each suggestions as s, i (s.handle)}
                <button type="button" class="suggestion-item" class:selected={i === selectedIndex} onclick={() => selectSuggestion(s.handle)}>
                  {#if s.avatar}
                    <img src={s.avatar} class="suggestion-avatar" alt="" />
                  {:else}
                    <div class="suggestion-avatar-placeholder"></div>
                  {/if}
                  <div class="suggestion-info">
                    <span class="suggestion-name">{s.displayName || s.handle}</span>
                    <span class="suggestion-handle">@{s.handle}</span>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <button type="submit">sign in</button>
    </form>
    <div class="login-faq">
      <button type="button" class="faq-toggle" onclick={() => toggleFaq('handle')}>
        <span>what is an internet handle?</span>
        <ChevronDown size={16} style={faqOpen.handle ? 'transform: rotate(180deg)' : ''} />
      </button>
      {#if faqOpen.handle}
        <div class="faq-content">
          <p>
            your internet handle is a domain that identifies you across apps built on
            <a href="https://atproto.com" target="_blank" rel="noopener">AT Protocol</a>.
            if you signed up for Bluesky or another ATProto service, you already have one
            (like <code>yourname.bsky.social</code>).
          </p>
          <p>read more at <a href="https://internethandle.org" target="_blank" rel="noopener">internethandle.org</a>.</p>
        </div>
      {/if}
      <button type="button" class="faq-toggle" onclick={() => toggleFaq('signup')}>
        <span>don't have one?</span>
        <ChevronDown size={16} style={faqOpen.signup ? 'transform: rotate(180deg)' : ''} />
      </button>
      {#if faqOpen.signup}
        <div class="faq-content">
          <p>
            the easiest way to get one is to sign up for <a href="https://bsky.app" target="_blank" rel="noopener">Bluesky</a>.
            once you have an account, you can use that handle here.
          </p>
        </div>
      {/if}
    </div>
  </div>
</div>
