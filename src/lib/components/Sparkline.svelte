<script lang="ts">
  let {
    values,
    color = 'var(--di-blue)',
    max = null,
    class: className = 'h-8 w-24',
  }: {
    values: number[];
    color?: string;
    max?: number | null;
    class?: string;
  } = $props();

  const ceiling = $derived(Math.max(1, max ?? Math.max(...values, 1)));
  const points = $derived(
    values.map((value, index) => {
      const x = values.length <= 1 ? 100 : (index / (values.length - 1)) * 100;
      const y = 30 - Math.max(0, Math.min(1, value / ceiling)) * 28;
      return [x, y] as const;
    }),
  );
  const line = $derived(points.map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`).join(' '));
  const area = $derived(points.length ? `0,32 ${line} 100,32` : '');
</script>

<svg
  class="{className} metric-spark"
  viewBox="0 0 100 32"
  preserveAspectRatio="none"
  fill="none"
  style:color={color}
  aria-hidden="true"
>
  {#if points.length > 1}
    <polygon points={area} fill="currentColor" opacity="0.12" />
    <polyline
      points={line}
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
      vector-effect="non-scaling-stroke"
    />
  {:else}
    <line x1="0" y1="20" x2="100" y2="20" stroke="currentColor" stroke-width="1.2" opacity="0.2" />
  {/if}
</svg>
