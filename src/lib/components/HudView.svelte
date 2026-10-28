<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';

  const overlay = $derived(island.overlay);
  const keyIcons: Record<string, 'tab' | 'enter' | 'win' | 'caps'> = {
    Tab: 'tab',
    Enter: 'enter',
    Win: 'win',
    'Caps Lock': 'caps',
  };
</script>

<div class="flex h-full w-full items-center gap-2.5 px-3.5" aria-hidden="true">
  {#if overlay?.kind === 'key'}
    <span
      class="flex size-7 shrink-0 items-center justify-center rounded-[9px] bg-[color:color-mix(in_srgb,var(--di-blue)_14%,transparent)] ring-1 ring-[color:color-mix(in_srgb,var(--di-blue)_28%,transparent)]"
    >
      <Icon name={keyIcons[overlay.key] ?? 'keyboard'} class="size-3.5 text-[var(--di-blue)]" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col gap-[4px] leading-none">
      <span class="truncate text-[13px] font-semibold">{overlay.key}</span>
      <span class="text-[8px] font-semibold tracking-[0.09em] text-[var(--faint)] uppercase">
        Tombol ditekan
      </span>
    </span>
  {:else if overlay?.kind === 'volume'}
    <span
      class="flex size-7 shrink-0 items-center justify-center rounded-[9px] bg-[color:color-mix(in_srgb,var(--di-blue)_14%,transparent)] ring-1 ring-[color:color-mix(in_srgb,var(--di-blue)_28%,transparent)]"
    >
      <Icon name={overlay.muted ? 'mute' : 'volume'} class="size-3.5 text-[var(--di-blue)]" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col gap-[4px] leading-none">
      <span class="flex items-baseline justify-between gap-2">
        <span class="text-[8px] font-semibold tracking-[0.09em] text-[var(--faint)] uppercase">
          {overlay.muted ? 'Bisu' : 'Volume'}
        </span>
        <span class="text-[15px] font-semibold tabular-nums">
          {overlay.muted ? 0 : overlay.percent}%
        </span>
      </span>
      <span class="relative h-[3px] w-full overflow-hidden rounded-full bg-[var(--track)]">
        <span
          class="absolute inset-y-0 left-0 rounded-full bg-[var(--di-blue)] transition-[width] duration-200"
          style:width="{overlay.muted ? 0 : overlay.percent}%"
        ></span>
      </span>
    </span>
  {/if}
</div>
