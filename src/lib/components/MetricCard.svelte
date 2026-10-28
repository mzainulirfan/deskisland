<script lang="ts">
  import Sparkline from './Sparkline.svelte';

  let {
    label,
    value,
    sub = '',
    color,
    history,
    max = 100,
  }: {
    label: string;
    value: string;
    sub?: string;
    color: string;
    history: number[];
    max?: number;
  } = $props();

  // 24 samples ≈ the last 60 s at the default 2.5 s poll; enough for a readable
  // silhouette without the chart turning into noise.
  const shown = $derived(history.slice(-24));
  const peak = $derived(shown.length ? Math.round(Math.max(...shown)) : 0);
  const mean = $derived(
    shown.length ? Math.round(shown.reduce((total, v) => total + v, 0) / shown.length) : 0,
  );
</script>

<div
  class="metric relative flex min-h-0 min-w-0 flex-col rounded-[13px] bg-[var(--card)] p-2.5 ring-1 ring-[color:var(--line)]"
  style:--tint={color}
>
  <div class="relative flex items-baseline justify-between gap-2">
    <span
      class="truncate text-[9px] font-semibold tracking-[0.14em] uppercase"
      style:color={color}
    >
      {label}
    </span>
    <span class="shrink-0 text-[19px] font-bold leading-none tracking-tight tabular-nums">
      {value}
    </span>
  </div>
  <span class="relative mt-0.5 truncate text-[10px] tabular-nums text-[var(--muted)]">{sub}</span>

  <div class="relative mt-1.5 min-h-[30px] flex-1">
    <Sparkline values={shown} {color} {max} class="h-full w-full" />
  </div>

  <div class="relative mt-1 flex items-center justify-between gap-2 text-[9px] tabular-nums text-[var(--faint)]">
    <span>rerata {mean}%</span>
    <span>puncak {peak}%</span>
  </div>
</div>

<style>
  /* A wash of the metric colour in the top-right corner makes the four cards read as
     one family without needing a border each. */
  .metric::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: radial-gradient(
      110% 90% at 100% 0%,
      color-mix(in srgb, var(--tint) 15%, transparent),
      transparent 62%
    );
  }

  .metric {
    --tint: var(--accent);
  }
</style>
