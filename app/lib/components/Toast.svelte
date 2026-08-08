<script lang="ts">
  import { fade } from 'svelte/transition'
  import { toast } from '$lib/toast.svelte'

  const icons: Record<string, string> = {
    success: '✓',
    error: '✕',
  }
</script>

{#if toast.toasts.length > 0}
  <div class="toast-container" role="region" aria-live="polite" aria-label="notifications">
    {#each toast.toasts as item (item.id)}
      <div
        class="toast toast-{item.type}"
        role="alert"
        transition:fade={{ duration: 400, easing: (t) => t * t }}
      >
        <span class="toast-icon" aria-hidden="true">{icons[item.type]}</span>
        <span class="toast-message">{item.message}</span>
      </div>
    {/each}
  </div>
{/if}
