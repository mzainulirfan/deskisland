<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { setMasterMute, setVolume } from '$lib/services/desktop';

  const volume = $derived(island.volume);
  const muted = $derived(volume?.muted ?? false);
  const device = $derived(volume?.device || 'Perangkat bawaan');

  let draft = $state<number | null>(null);
  const shown = $derived(draft ?? (muted ? 0 : (volume?.percent ?? 0)));
  const mutedShown = $derived(draft === null && muted);

  // The Windows volume API is a COM round-trip per write, so drag input is coalesced:
  // one write in flight, at most one every SEND_MS, always carrying the newest value.
  const SEND_MS = 55;
  let pumping = false;
  let lastSentAt = 0;
  let settle: ReturnType<typeof setTimeout> | undefined;

  function beginEdit() {
    clearTimeout(settle);
    island.volumeEditing = true;
  }

  function settleEdit() {
    clearTimeout(settle);
    settle = setTimeout(() => {
      island.volumeEditing = false;
    }, 200);
  }

  async function pump() {
    if (pumping) return;
    pumping = true;
    try {
      for (;;) {
        const target = draft;
        if (target === null) break;
        const wait = SEND_MS - (Date.now() - lastSentAt);
        if (wait > 0) await new Promise<void>((resolve) => setTimeout(resolve, wait));
        if (draft !== target) continue;
        lastSentAt = Date.now();
        const next = await setVolume(target);
        if (next) island.volume = next;
        if (draft === target) draft = null;
      }
    } finally {
      pumping = false;
      if (draft === null) settleEdit();
    }
  }

  function onInput(event: Event) {
    const percent = Number((event.currentTarget as HTMLInputElement).value);
    beginEdit();
    draft = percent;
    const current = island.volume;
    if (current?.muted) island.volume = { ...current, muted: false };
    void pump();
  }

  async function toggleMute() {
    const current = island.volume;
    if (!current) return;
    beginEdit();
    draft = null;
    island.volume = { ...current, muted: !current.muted };
    try {
      const next = await setMasterMute(!current.muted);
      if (next) island.volume = next;
    } finally {
      settleEdit();
    }
  }
</script>

<!-- Sits inside the shared audio shell, so this section carries no card of its own. -->
<div class="flex h-full min-w-0 flex-col justify-center">
  <div class="flex items-center gap-2">
    <button
      type="button"
      class="flex size-7 shrink-0 items-center justify-center rounded-full transition-colors disabled:opacity-40 {mutedShown
        ? 'bg-[var(--chip)] text-[var(--faint)] ring-1 ring-[color:var(--line)] hover:text-[var(--ink)]'
        : 'bg-[color:color-mix(in_srgb,var(--di-blue)_16%,transparent)] text-[var(--di-blue)] ring-1 ring-[color:color-mix(in_srgb,var(--di-blue)_28%,transparent)] hover:bg-[color:color-mix(in_srgb,var(--di-blue)_24%,transparent)]'}"
      aria-label={mutedShown ? 'Aktifkan suara' : 'Bisukan suara'}
      aria-pressed={mutedShown}
      title={mutedShown ? 'Aktifkan suara' : 'Bisukan suara'}
      disabled={!volume}
      onclick={() => void toggleMute()}
    >
      <Icon name={mutedShown ? 'mute' : 'volume'} class="size-4" />
    </button>
    <span class="text-[9px] font-semibold tracking-[0.12em] text-[var(--muted)] uppercase">
      Volume
    </span>
    <span class="ml-auto shrink-0 text-[17px] font-bold leading-none tabular-nums {mutedShown
      ? 'text-[var(--faint)]'
      : 'text-[var(--di-blue)]'}">
      {volume ? (mutedShown ? 'Bisu' : `${Math.round(shown)}%`) : '—'}
    </span>
  </div>

  <input
    type="range"
    class="volume-range mt-2"
    class:is-muted={mutedShown}
    min="0"
    max="100"
    step="1"
    value={Math.round(shown)}
    disabled={!volume}
    aria-label="Volume sistem"
    aria-valuetext={mutedShown ? 'Bisu' : `${Math.round(shown)} persen`}
    style="--fill: {Math.round(shown)}%"
    oninput={onInput}
  />

  <div class="mt-1 flex items-center justify-between gap-2 text-[9.5px]">
    <span class="min-w-0 truncate text-[var(--faint)]" title={device}>{device}</span>
    <span class="shrink-0 {mutedShown ? 'text-[var(--faint)]' : 'text-[var(--di-blue)]'}">
      {mutedShown ? 'Bisu' : 'Aktif'}
    </span>
  </div>
</div>

<style>
  .volume-range {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 18px;
    background: transparent;
    cursor: pointer;
  }

  .volume-range:disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }

  .volume-range::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 999px;
    background: linear-gradient(
      to right,
      var(--di-blue) 0%,
      var(--di-blue) var(--fill, 0%),
      var(--track) var(--fill, 0%),
      var(--track) 100%
    );
  }

  .volume-range::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 999px;
    border: 2px solid var(--di-blue);
    background: var(--di-surface);
    box-shadow: 0 1px 4px rgb(2 6 23 / 0.35);
    transition: transform 120ms ease;
  }

  .volume-range:hover::-webkit-slider-thumb {
    transform: scale(1.12);
  }

  .volume-range.is-muted::-webkit-slider-thumb {
    border-color: var(--faint);
  }

  .volume-range:active::-webkit-slider-thumb {
    transform: scale(1.18);
  }

  .volume-range:focus-visible {
    outline: none;
  }

  .volume-range:focus-visible::-webkit-slider-thumb {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--di-blue) 30%, transparent);
  }
</style>
