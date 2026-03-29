<script lang="ts">
  import type { Snippet } from 'svelte'
  import '../app.css'
  import '$lib/auth'
  import Header from '$lib/components/Header.svelte'
  import { QueryClientProvider } from '@tanstack/svelte-query'
  import { loadPreferences } from '$lib/preferences'

  let { data, children }: { data: any; children: Snippet } = $props()

  $effect(() => {
    if (data.preferences) {
      Promise.resolve(data.preferences).then((prefs: any) => loadPreferences(prefs?.preferences ?? prefs))
    }
  })
</script>

<QueryClientProvider client={data.queryClient}>
  <div class="app-shell">
    <Header />
    {@render children()}
  </div>
</QueryClientProvider>
