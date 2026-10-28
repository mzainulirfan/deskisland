<script lang="ts">
  import Icon, { type IconName } from './Icon.svelte';
  import Sparkline from './Sparkline.svelte';

  let {
    label,
    icon,
    color,
    value,
    sub = '',
    history = null,
    max = 100,
    progress = null,
    tone = 'text-[var(--ink)]',
  }: {
    label: string;
    icon: IconName;
    color: string;
    value: string;
    sub?: string;
    history?: number[] | null;
    max?: number;
    progress?: number | null;
    tone?: string;
  } = $props();

  const fill = $derived(Math.max(0, Math.min(100, progress ?? 0)));
</script>

<div
  class="tile relative flex min-h-0 min-w-0 flex-col rounded-[13px] bg-[var(--card)] p-2.5 ring-1 ring-[color:var(--line)]"
  style:--tint={color}
>
  <div class="relative flex items-center gap-1.5">
    <span class="flex size-4 shrink-0 items-center justify-center" style:color={color}>
      <Icon name={icon} class="size-3.5" />
    </span>
    <span
      class="min-w-0 flex-1 truncate text-[9px] font-semibold tracking-[0.12em] text-[var(--muted)] uppercase"
    >
      {label}
    </span>
    <span class="shrink-0 text-[13px] font-bold leading-none tabular-nums {tone}">{value}</span>
  </div>

  {#if history}
    <div class="relative mt-1 min-h-[18px] flex-1">
      <Sparkline values={history} {color} {max} class="h-full w-full" />
    </div>
  {:else}
    <div class="relative mt-auto flex min-h-[18px] flex-col justify-center">
      <span class="block h-[5px] w-full overflow-hidden rounded-full bg-[var(--track)]">
        <span
          class="block h-full rounded-full transition-[width] duration-500"
          style:width="{fill}%"
          style:background={color}
        ></span>
      </span>
    </div>
  {/if}

  <span class="relative mt-0.5 truncate text-[9.5px] tabular-nums text-[var(--faint)]">{sub}</span>
</div>

<style>
  /* A wash of the metric colour keeps the small tiles in the same family as the
     larger metric cards. */
  .tile::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--tint) 11%, transparent),
      transparent 60%
    );
  }
</style>
