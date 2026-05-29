<script lang="ts">
  import { page } from '$app/stores'
  import { createQuery } from '@tanstack/svelte-query'
  import { actorFeedQuery } from '$lib/queries'
  import { atprotoClient, resolveClient } from '$lib/atclients'
  import { logout } from '$lib/auth'
  import { House, Rss, Settings, LogOut } from 'lucide-svelte'
  import SettingsModal from './SettingsModal.svelte'

  let settingsOpen = $state(false)

  const viewer = $derived($page.data.viewer)
  const currentPage = $derived($page.url.pathname)
  const client = $derived(resolveClient($atprotoClient))

  // resolve the @handle for the profile title from the already-cached feed query
  const profileDid = $derived(
    currentPage.startsWith('/profile/') ? decodeURIComponent(currentPage.slice('/profile/'.length)) : null
  )
  const profileFeed = createQuery(() => ({
    ...actorFeedQuery(profileDid ?? ''),
    enabled: !!profileDid,
  }))
  const profileHandle = $derived(profileFeed.data?.items?.[0]?.handle ?? profileDid?.slice(0, 18) ?? '')

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
      <a href={client.profileUrl(viewer.handle ?? viewer.did)} target="_blank" rel="noopener">
        @{viewer.handle ?? viewer.did.slice(0, 18)}
      </a>
    {:else if currentPage.startsWith('/feed')}
      global feed
    {:else if currentPage.startsWith('/@')}
      {@const handle = decodeURIComponent(currentPage.slice(2))}
      <a href={client.profileUrl(handle)} target="_blank" rel="noopener">@{handle}</a>
    {:else if currentPage.startsWith('/profile/')}
      <a href={client.profileUrl(profileHandle)} target="_blank" rel="noopener">@{profileHandle}</a>
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
      <button class="nav-btn" onclick={handleLogout} aria-label="log out" title="log out">
        <LogOut size={20} />
      </button>
    {/if}
  </nav>
</header>

{#if settingsOpen}
  <SettingsModal onclose={() => settingsOpen = false} />
{/if}
