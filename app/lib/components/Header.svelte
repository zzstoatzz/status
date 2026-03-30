<script lang="ts">
  import { page } from '$app/stores'
  import { logout } from '$lib/auth'
  import { House, Rss, Settings, LogOut } from 'lucide-svelte'
  import SettingsModal from './SettingsModal.svelte'

  let settingsOpen = $state(false)

  const viewer = $derived($page.data.viewer)
  const currentPage = $derived($page.url.pathname)

  function toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme')
    const next = current === 'dark' ? 'light' : 'dark'
    document.documentElement.setAttribute('data-theme', next)
    localStorage.setItem('theme', next)
  }

  async function handleLogout() {
    await logout()
    window.location.href = '/'
  }

  function initTheme() {
    if (typeof document === 'undefined') return
    const saved = localStorage.getItem('theme') || 'dark'
    document.documentElement.setAttribute('data-theme', saved)
  }

  $effect(() => {
    initTheme()
  })
</script>

<header>
  <h1>
    {#if currentPage === '/' && viewer}
      <a href="https://bsky.app/profile/{viewer.handle ?? viewer.did}" target="_blank">
        @{viewer.handle ?? viewer.did.slice(0, 18)}
      </a>
    {:else if currentPage.startsWith('/feed')}
      global feed
    {:else if currentPage.startsWith('/@')}
      {@const handle = decodeURIComponent(currentPage.slice(2))}
      <a href="https://bsky.app/profile/{handle}" target="_blank">@{handle}</a>
    {:else if currentPage.startsWith('/profile/')}
      profile
    {:else}
      status
    {/if}
  </h1>
  <nav>
    {#if currentPage !== '/'}
      <a href="/" class="nav-btn" aria-label="home" title="home">
        <House size={20} />
      </a>
    {/if}
    {#if currentPage !== '/feed'}
      <a href="/feed" class="nav-btn" aria-label="feed" title="global feed">
        <Rss size={20} />
      </a>
    {/if}
    {#if viewer}
      <button class="nav-btn" onclick={() => settingsOpen = true} aria-label="settings" title="settings">
        <Settings size={20} />
      </button>
    {/if}
    <button class="theme-toggle" onclick={toggleTheme} aria-label="toggle theme">
      <span class="sun">☀</span><span class="moon">☾</span>
    </button>
    {#if viewer}
      <button class="nav-btn" onclick={handleLogout} aria-label="log out" title="log out">
        <LogOut size={20} />
      </button>
    {/if}
  </nav>
</header>

{#if settingsOpen}
  <SettingsModal onclose={() => settingsOpen = false} />
{/if}

<style>
  .sun { display: none; }
  .moon { display: inline; }
  :global([data-theme="light"]) .sun { display: inline; }
  :global([data-theme="light"]) .moon { display: none; }
</style>
