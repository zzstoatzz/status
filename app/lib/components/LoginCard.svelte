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
  let signingIn = $state(false)
  let loginError = $state('')

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

  async function selectSuggestion(h: string) {
    handle = h
    showDropdown = false
    suggestions = []
    await startLogin(h)
  }

  async function startLogin(h: string) {
    const trimmed = h.trim()
    if (!trimmed || signingIn) return
    signingIn = true
    try {
      await login(trimmed)
    } catch (err: any) {
      signingIn = false
      loginError = err?.message ?? 'could not start sign in. try again.'
    }
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
    await startLogin(handle)
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
        <label for="handle-input">atmosphere account</label>
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
                <button type="button" class="suggestion-item" class:selected={i === selectedIndex} onpointerdown={(e) => { e.preventDefault(); selectSuggestion(s.handle) }}>
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
      {#if loginError}<p class="login-error" role="alert">{loginError}</p>{/if}
      <button type="submit" disabled={signingIn}>{signingIn ? 'signing in…' : 'sign in'}</button>
    </form>
    <div class="login-faq">
      <button type="button" class="faq-toggle" onclick={() => toggleFaq('handle')}>
        <span>what's an atmosphere account?</span>
        <ChevronDown size={16} style={faqOpen.handle ? 'transform: rotate(180deg)' : ''} />
      </button>
      {#if faqOpen.handle}
        <div class="faq-content">
          <p>
            one account for every app built on the
            <a href="https://atproto.com" target="_blank" rel="noopener">atmosphere</a>.
            you sign in with your handle — a domain like <code>yourname.bsky.social</code>
            — and your data stays yours, wherever you use it.
          </p>
          <p>
            if you've signed up for <a href="https://bsky.app" target="_blank" rel="noopener">Bluesky</a>
            or any other atproto app, you already have one.
          </p>
        </div>
      {/if}
      <button type="button" class="faq-toggle" onclick={() => toggleFaq('signup')}>
        <span>don't have one yet?</span>
        <ChevronDown size={16} style={faqOpen.signup ? 'transform: rotate(180deg)' : ''} />
      </button>
      {#if faqOpen.signup}
        <div class="faq-content">
          <p>
            you can't make one here yet. the quickest route is
            <a href="https://bsky.app" target="_blank" rel="noopener">Bluesky</a> — sign up there,
            then come back and use that handle.
          </p>
        </div>
      {/if}
    </div>
  </div>
</div>
