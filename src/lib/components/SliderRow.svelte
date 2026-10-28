<script lang="ts">
  import Icon, { type IconName } from './Icon.svelte';

  let {
    icon,
    label,
    percent = null,
    value,
    sub = '',
    onchange = null,
  }: {
    icon: IconName;
    label: string;
    percent?: number | null;
    value: string;
    sub?: string;
    onchange?: ((percent: number) => void) | null;
  } = $props();

  let track = $state<HTMLButtonElement | null>(null);
  let dragging = false;

  const fill = $derived(percent === null ? 0 : Math.max(0, Math.min(100, percent)));

  function apply(clientX: number) {
    if (!track || !onchange) return;
    const rect = track.getBoundingClientRect();
    onchange(Math.max(0, Math.min(100, Math.round(((clientX - rect.left) / rect.width) * 100))));
  }
</script>

<div class="flex {sub ? 'h-7' : 'h-6'} items-center gap-2">
  <Icon name={icon} class="size-3.5 text-[var(--muted)]" />
  <span class="w-[52px] shrink-0 text-[10px] tracking-wide text-[var(--muted)] uppercase">{label}</span>

  {#if onchange}
    <button
      type="button"
      bind:this={track}
      class="flex h-4 min-w-0 flex-1 cursor-pointer items-center outline-none"
      role="slider"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(fill)}
      onpointerdown={(event) => {
        dragging = true;
        apply(event.clientX);
        try {
          track?.setPointerCapture(event.pointerId);
        } catch {
          // A synthetic or already-released pointer cannot be captured; dragging still works.
        }
      }}
      onpointermove={(event) => {
        if (dragging) apply(event.clientX);
      }}
      onpointerup={() => {
        dragging = false;
      }}
    >
      <span class="relative h-1.5 w-full rounded-full bg-[var(--track)]">
        {#if percent !== null}
          <span
            class="absolute inset-y-0 left-0 rounded-full bg-[var(--accent)] transition-[width] duration-300"
            style:width="{fill}%"
          ></span>
          <span
            class="absolute top-1/2 size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-[var(--accent)] ring-2 ring-[color:var(--surface)]"
            style:left="clamp(5px, {fill}%, calc(100% - 5px))"
          ></span>
        {/if}
      </span>
    </button>
  {:else}
    <span class="flex h-4 min-w-0 flex-1 items-center">
      <span class="relative h-1.5 w-full rounded-full bg-[var(--track)]">
        {#if percent !== null}
          <span
            class="absolute inset-y-0 left-0 rounded-full bg-[var(--accent)]"
            style:width="{fill}%"
          ></span>
        {/if}
      </span>
    </span>
  {/if}

  <span class="w-[84px] shrink-0 text-right leading-tight">
    <span class="block text-[11px] tabular-nums">{value}</span>
    {#if sub}
      <span class="block truncate text-[9px] tabular-nums text-[var(--faint)]">{sub}</span>
    {/if}
  </span>
</div>
