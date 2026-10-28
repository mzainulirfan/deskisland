<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { getNowPlaying, mediaCommand, type MediaAction } from '$lib/services/desktop';

  const media = $derived(island.media);
  const playing = $derived(media?.playing ?? false);
  const progress = $derived(
    media && media.duration > 0
      ? Math.max(0, Math.min(100, (media.position / media.duration) * 100))
      : 0,
  );

  let busy = $state(false);
  let optimistic = $state<boolean | null>(null);
  const shownPlaying = $derived(optimistic ?? playing);

  async function command(action: MediaAction) {
    if (!media || busy) return;
    busy = true;
    // Play/pause flips immediately; the 220 ms round-trip is otherwise felt as lag.
    if (action === 'toggle') optimistic = !media.playing;
    else if (action === 'play') optimistic = true;
    else if (action === 'pause') optimistic = false;
    try {
      const ok = await mediaCommand(action);
      if (!ok) return;
      await new Promise((resolve) => setTimeout(resolve, 220));
      const next = await getNowPlaying();
      if (next) island.media = next;
    } finally {
      optimistic = null;
      busy = false;
    }
  }

  function clock(seconds: number) {
    const total = Math.max(0, Math.floor(seconds));
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const secs = total % 60;
    return hours > 0
      ? `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
      : `${minutes}:${String(secs).padStart(2, '0')}`;
  }
</script>

<!-- Sits inside the shared audio shell, so this section carries no card of its own. -->
<div class="flex h-full min-w-0 items-center gap-3">
  <div
    class="relative flex size-[58px] shrink-0 items-center justify-center overflow-hidden rounded-[13px] ring-1 ring-[color:var(--line)] {media
      ? 'bg-[radial-gradient(circle_at_30%_20%,color-mix(in_srgb,var(--di-purple)_46%,transparent),transparent_70%),linear-gradient(145deg,var(--di-surface-hover),var(--di-bg))]'
      : 'bg-[radial-gradient(circle_at_40%_30%,var(--chip),var(--di-bg))]'}"
  >
    <Icon name="media" class="size-5 {media ? 'text-[var(--di-purple)]' : 'text-[var(--faint)]'}" />
    {#if shownPlaying}
      <span
        class="absolute inset-x-2 bottom-1.5 flex h-2 items-end justify-center gap-[3px] text-[var(--di-purple)]"
      >
        {#each [0, 1, 2, 3] as bar}
          <span
            class="eq-bar w-[2px] rounded-full bg-current"
            style:height="{4 + bar * 1.5}px"
            style:animation-delay="{bar * 120}ms"
          ></span>
        {/each}
      </span>
    {/if}
  </div>

  <div class="flex min-w-0 flex-1 flex-col justify-center">
    <span class="truncate text-[9px] font-semibold tracking-[0.12em] text-[var(--di-purple)] uppercase">
      {media ? media.app || 'Media' : 'Tidak ada pemutaran'}
    </span>
    <span class="mt-0.5 truncate text-[15px] font-semibold leading-tight tracking-tight" title={media?.title}>
      {media ? media.title || 'Tanpa judul' : 'Putar musik di aplikasi mana pun'}
    </span>
    <span class="truncate text-[11px] text-[var(--muted)]" title={media?.artist}>
      {media ? media.artist || 'Tanpa artis' : 'Kontrol media muncul di sini'}
    </span>

    {#if media}
      <div class="mt-1.5 flex items-center gap-2">
        <span class="w-8 shrink-0 text-[9px] tabular-nums text-[var(--faint)]">
          {clock(media.position)}
        </span>
        <span class="relative h-[4px] min-w-0 flex-1 overflow-hidden rounded-full bg-[var(--track)]">
          <span
            class="absolute inset-y-0 left-0 rounded-full bg-[var(--di-purple)] transition-[width] duration-500"
            style:width="{progress}%"
          ></span>
        </span>
        <span class="w-8 shrink-0 text-right text-[9px] tabular-nums text-[var(--faint)]">
          {media.duration > 0 ? clock(media.duration) : '0:00'}
        </span>
      </div>
    {/if}
  </div>

  <div class="flex shrink-0 items-center gap-1.5">
    <button
      type="button"
      class="flex size-7 items-center justify-center rounded-full bg-[var(--chip)] text-[var(--ink)] ring-1 ring-[color:var(--line)] transition-colors hover:bg-[var(--di-surface-hover)] disabled:opacity-35"
      aria-label="Lagu sebelumnya"
      title="Lagu sebelumnya"
      disabled={!media || busy}
      onclick={() => void command('previous')}
    >
      <Icon name="prev" class="size-3.5" />
    </button>

    <button
      type="button"
      class="flex size-10 items-center justify-center rounded-full bg-[var(--di-purple)] text-[#0b0d12] shadow-[0_0_18px_color-mix(in_srgb,var(--di-purple)_45%,transparent)] transition-transform hover:scale-105 disabled:opacity-35 disabled:hover:scale-100"
      aria-label={shownPlaying ? 'Jeda' : 'Putar'}
      title={shownPlaying ? 'Jeda' : 'Putar'}
      disabled={!media || busy}
      onclick={() => void command('toggle')}
    >
      <Icon name={shownPlaying ? 'pause' : 'play'} class="size-4" />
    </button>

    <button
      type="button"
      class="flex size-7 items-center justify-center rounded-full bg-[var(--chip)] text-[var(--ink)] ring-1 ring-[color:var(--line)] transition-colors hover:bg-[var(--di-surface-hover)] disabled:opacity-35"
      aria-label="Lagu berikutnya"
      title="Lagu berikutnya"
      disabled={!media || busy}
      onclick={() => void command('next')}
    >
      <Icon name="next" class="size-3.5" />
    </button>
  </div>
</div>
