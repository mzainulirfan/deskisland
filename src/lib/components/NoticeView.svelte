<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { cancelTimer } from '$lib/services/desktop';

  const overlay = $derived(island.overlay);

  function size(bytes: number) {
    const mb = bytes / 1024 ** 2;
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(1)} MB`;
  }

  function minutes(ms: number) {
    const total = Math.round(ms / 60_000);
    if (total < 60) return `${total} menit`;
    const hours = Math.floor(total / 60);
    return total % 60 ? `${hours} jam ${total % 60} menit` : `${hours} jam`;
  }
</script>

<div class="flex h-full w-full items-center gap-3 px-4">
  {#if overlay?.kind === 'notification'}
    <span
      class="flex size-7 shrink-0 items-center justify-center rounded-[10px] bg-[color:color-mix(in_srgb,var(--di-blue)_14%,transparent)] ring-1 ring-[color:color-mix(in_srgb,var(--di-blue)_22%,transparent)]"
    >
      <Icon name="bell" class="size-4 text-[var(--di-blue)]" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col leading-tight">
      <span class="truncate text-[12px] font-semibold">{overlay.title}</span>
      <span class="truncate text-[10px] text-[var(--muted)]">
        {overlay.app}{overlay.body ? ` · ${overlay.body}` : ''}
      </span>
    </span>
  {:else if overlay?.kind === 'download'}
    <span
      class="flex size-7 shrink-0 items-center justify-center rounded-[10px] bg-[color:color-mix(in_srgb,var(--di-green)_14%,transparent)] ring-1 ring-[color:color-mix(in_srgb,var(--di-green)_22%,transparent)]"
    >
      <Icon name="download" class="size-4 text-[var(--di-green)]" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col leading-tight">
      <span class="truncate text-[12px] font-semibold">{overlay.name}</span>
      <span class="truncate text-[10px] text-[var(--muted)]">
        Terunduh · {size(overlay.size)} · klik untuk aksi
      </span>
    </span>
  {:else if overlay?.kind === 'reminder'}
    <span
      class="flex size-7 shrink-0 items-center justify-center rounded-[10px] bg-[color:color-mix(in_srgb,var(--di-amber)_14%,transparent)] ring-1 ring-[color:color-mix(in_srgb,var(--di-amber)_22%,transparent)]"
    >
      <Icon name="bell" class="size-4 text-[var(--di-amber)]" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col leading-tight">
      <span class="truncate text-[12px] font-semibold">{overlay.text}</span>
      <span class="truncate text-[10px] text-[var(--muted)]">
        Pengingat {overlay.time} · klik untuk buka
      </span>
    </span>
  {:else if overlay?.kind === 'timer'}
    {@const isPomoWork = overlay.label.toLowerCase().includes('fokus')}
    {@const isPomoBreak = overlay.label.toLowerCase().includes('istirahat')}
    {@const accentColor = isPomoWork
      ? 'var(--di-red)'
      : isPomoBreak
        ? 'var(--di-green)'
        : 'var(--di-amber)'}
    <span
      class="flex size-7 shrink-0 items-center justify-center rounded-[10px] ring-1"
      class:island-pulse={island.timerAlarming}
      style:background="color-mix(in srgb, {accentColor} 16%, transparent)"
      style:border-color="color-mix(in srgb, {accentColor} 26%, transparent)"
      style:color={accentColor}
    >
      <Icon name="timer" class="size-4" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col leading-tight">
      <span class="truncate text-[12px] font-semibold" style:color={accentColor}
        >Waktu habis · {overlay.label}</span
      >
      <span class="truncate text-[10px] text-[var(--muted)]">
        {minutes(overlay.total)} berlalu · klik untuk buka
      </span>
    </span>
    <button
      type="button"
      class="flex size-6 shrink-0 items-center justify-center rounded-full bg-[var(--chip)] text-[var(--muted)] ring-1 ring-[color:var(--line)] transition-colors hover:bg-[var(--di-surface-hover)] hover:text-[var(--ink)]"
      aria-label="Tutup notifikasi timer"
      title="Tutup notifikasi"
      onclick={(event) => {
        event.stopPropagation();
        island.dismissTimerAlarm();
        void cancelTimer().then((info) => {
          if (info) island.setTimer(info);
        });
      }}
    >
      <Icon name="close" class="size-3" />
    </button>
  {/if}
</div>
