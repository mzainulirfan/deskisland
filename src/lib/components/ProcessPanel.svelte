<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { openTaskManager } from '$lib/services/desktop';

  const processes = $derived(island.processes.slice(0, 4));
  const peak = $derived(Math.max(1, ...processes.map((process) => process.cpu)));

  function memory(bytes: number) {
    const mb = bytes / 1024 ** 2;
    return mb >= 1024
      ? `${(mb / 1024).toFixed(1).replace('.', ',')} GB`
      : `${Math.round(mb)} MB`;
  }

  /// Bars are relative to the busiest process so the ranking stays readable even when
  /// every process sits in the low single digits; the percent label keeps the absolute.
  const bar = (cpu: number) => Math.max(4, Math.min(100, (cpu / peak) * 100));
</script>

<div
  class="flex h-full flex-col rounded-[16px] bg-[var(--card)] p-3 ring-1 ring-[color:var(--line)]"
>
  <header class="flex h-5 shrink-0 items-center justify-between gap-3">
    <h2
      class="flex items-center gap-1.5 text-[9px] font-medium tracking-[0.08em] text-[var(--muted)] uppercase"
    >
      <span class="size-1.5 rounded-full bg-[var(--di-amber)]"></span>
      Proses · CPU tertinggi
    </h2>

    <button
      type="button"
      class="flex h-5 items-center gap-1 rounded-full bg-[color:color-mix(in_srgb,var(--di-amber)_12%,transparent)] px-2 text-[9px] font-medium text-[var(--di-amber)] ring-1 ring-[color:color-mix(in_srgb,var(--di-amber)_22%,transparent)] transition-colors hover:bg-[color:color-mix(in_srgb,var(--di-amber)_20%,transparent)]"
      aria-label="Buka Task Manager"
      title="Buka Task Manager"
      onclick={() => void openTaskManager()}
    >
      <Icon name="external" class="size-2.5" />
      Task Manager
    </button>
  </header>

  {#if processes.length}
    <ol
      class="mt-1 flex min-h-0 flex-1 flex-col justify-center gap-[3px]"
      aria-label="Proses dengan CPU tertinggi"
    >
      {#each processes as process, index (process.name)}
        <li
          class="grid h-[21px] grid-cols-[12px_minmax(0,1fr)_48px_32px_52px] items-center gap-2 text-[11px]"
          title={`${process.name} · ${Math.round(process.cpu)}% CPU · ${memory(process.memory)}`}
        >
          <span class="text-right text-[9px] tabular-nums text-[var(--faint)]">{index + 1}</span>
          <span class="truncate font-medium">{process.name}</span>
          <span class="relative h-[3px] overflow-hidden rounded-full bg-[var(--track)]">
            <span
              class="absolute inset-y-0 left-0 rounded-full bg-[var(--di-amber)] transition-[width] duration-500"
              style:width="{bar(process.cpu)}%"
            ></span>
          </span>
          <span class="text-right text-[10px] tabular-nums text-[var(--muted)]">
            {Math.round(process.cpu)}%
          </span>
          <span class="text-right text-[10px] tabular-nums text-[var(--faint)]">
            {memory(process.memory)}
          </span>
        </li>
      {/each}
    </ol>
  {:else}
    <div class="flex min-h-0 flex-1 items-center justify-center gap-2 text-[10px] text-[var(--muted)]">
      <Icon name="activity" class="size-3.5" />
      Tidak ada proses aktif
    </div>
  {/if}
</div>
