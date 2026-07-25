<script lang="ts">
  import { onMount } from 'svelte'

  // hard navigation, not goto(): the session cookie was just set by the server on this
  // request, and a client-side nav can reuse the root layout's cached logged-out data.
  onMount(() => {
    window.location.replace('/')
  })
</script>

<p class="oauth-redirecting">signing you in…</p>
