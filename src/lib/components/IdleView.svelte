<script lang="ts">
  import Icon from './Icon.svelte';
  import Mascot from './Mascot.svelte';
  import { island } from '$lib/stores/island.svelte';

  const stats = $derived(island.stats);
  const variant = $derived(island.activeIdle);
  const media = $derived(island.media);
  const playing = $derived(island.media?.playing ?? false);

  const clockParts = $derived(island.time.split(/[.:]/));
  const ring = $derived(2 * Math.PI * 14.5);

  // With nothing running the pill arms itself with the last chosen length, so the timer
  // style still has something to show instead of a countdown that never fires.
  const timerLive = $derived(island.timerLive);
  const armedMs = $derived(island.timerMinutes * 60_000);
  const shownMs = $derived(timerLive ? island.timerLeft : armedMs);
  const timerTotal = $derived(Math.max(1000, timerLive ? (island.timer?.total ?? armedMs) : armedMs));
  const timerParts = $derived(mmss(shownMs));
  const ringOffset = $derived(ring * (1 - Math.min(1, shownMs / timerTotal)));
  const timerCaption = $derived(
    !timerLive
      ? 'Siap mulai'
      : island.timer?.status === 'paused'
        ? 'Dijeda'
        : island.timer?.label || 'Timer',
  );

  const isPomoWork = $derived(
    island.timer?.label?.toLowerCase().includes('fokus') ?? false,
  );
  const isPomoBreak = $derived(
    island.timer?.label?.toLowerCase().includes('istirahat') ?? false,
  );
  const timerAccent = $derived(
    island.timer?.status === 'paused'
      ? 'var(--faint)'
      : isPomoWork
        ? 'var(--di-red)'
        : isPomoBreak
          ? 'var(--di-green)'
          : 'var(--di-amber)',
  );

  const cpuTone = $derived(
    (stats?.cpu ?? 0) > 85
      ? 'var(--di-red)'
      : (stats?.cpu ?? 0) > 60
        ? 'var(--di-amber)'
        : 'var(--di-green)',
  );
  const netLoad = $derived(
    stats ? Math.min(100, ((stats.rx + stats.tx) / (1024 * 1024)) * 100) : 0,
  );

  const battery = $derived(stats?.battery ?? null);
  const batteryTone = $derived(
    battery == null
      ? 'text-[var(--faint)]'
      : battery.charging || battery.percent > 30
        ? 'text-[var(--di-green)]'
        : battery.percent > 15
          ? 'text-[var(--di-amber)]'
          : 'text-[var(--di-red)]',
  );

  let playPos = $state(0);
  const musicProgress = $derived(
    media && media.duration > 0 ? Math.max(0, Math.min(100, (playPos / media.duration) * 100)) : 0,
  );

  $effect(() => {
    playPos = island.media?.position ?? 0;
  });

  $effect(() => {
    if (variant !== 'music' || !media?.playing || !media.duration) return;
    const timer = setInterval(() => {
      playPos = Math.min(media.duration, playPos + 1);
    }, 1000);
    return () => clearInterval(timer);
  });

  let noticeIndex = $state(0);
  const notificationLog = $derived(island.notificationLog);
  const newestNotice = $derived(notificationLog[0]);
  const currentNotice = $derived(
    notificationLog.length ? notificationLog[noticeIndex % notificationLog.length] : null,
  );

  $effect(() => {
    // A new notification takes over immediately, then the list keeps rotating. Depend
    // on the newest entry, not the length: the log is capped, so a new item can arrive
    // while `length` stays unchanged.
    newestNotice;
    noticeIndex = 0;
  });

  $effect(() => {
    if (variant !== 'notification' || notificationLog.length < 2) return;
    const timer = setInterval(() => {
      noticeIndex = (noticeIndex + 1) % notificationLog.length;
    }, 4000);
    return () => clearInterval(timer);
  });

  function rate(bytesPerSecond: number) {
    if (bytesPerSecond >= 1024 ** 2) return `${(bytesPerSecond / 1024 ** 2).toFixed(1)} MB/s`;
    if (bytesPerSecond >= 1024) return `${Math.round(bytesPerSecond / 1024)} KB/s`;
    return `${Math.round(bytesPerSecond)} B/s`;
  }

  /// Split into the two numbers the pill prints either side of its accent dot. Rounding up
  /// keeps `01` on screen for the final second instead of jumping straight to zero.
  function mmss(ms: number) {
    const seconds = Math.ceil(ms / 1000);
    return [
      String(Math.floor(seconds / 60)).padStart(2, '0'),
      String(seconds % 60).padStart(2, '0'),
    ];
  }
</script>

<div
  class="idle-swap flex h-full w-full items-center gap-2.5 px-3.5"
  class:justify-center={variant === 'min'}
  aria-hidden="true"
>
  {#if variant === 'eyes'}
    <Mascot />

    <span class="ml-1 shrink-0 text-[20px] font-bold leading-none tracking-tight tabular-nums">
      {island.time}
    </span>

    <span class="ml-auto flex shrink-0 items-center gap-1.5">
      <span
        class="flex h-6 items-center gap-1.5 rounded-full bg-[var(--chip)] px-2 text-[10px] font-medium tabular-nums ring-1 ring-[color:var(--line)]"
        title="Beban CPU"
      >
        <span class="size-[5px] rounded-full" style:background={cpuTone}></span>
        {stats ? `${Math.round(stats.cpu)}%` : '—'}
      </span>
      <span
        class="flex h-6 items-center gap-1 rounded-full bg-[var(--chip)] px-2 text-[10px] tabular-nums text-[var(--muted)] ring-1 ring-[color:var(--line)]"
        title="Lalu lintas jaringan"
      >
        <Icon name="download" class="size-2.5 text-[var(--di-blue)]" />
        {stats ? rate(stats.rx) : '—'}
      </span>
    </span>
  {:else if variant === 'min'}
    <Mascot />
    <span class="text-[19px] font-semibold leading-none tracking-tight tabular-nums">
      {clockParts[0]}<span class="text-[var(--di-blue)]">.</span>{clockParts[1]}
    </span>
  {:else if variant === 'battery'}
    <span class="flex flex-1 items-center justify-center gap-3">
      <span class="relative flex shrink-0 items-center {batteryTone}">
        <span
          class="relative flex h-[15px] w-[28px] items-center rounded-[4px] border-[1.75px] border-current px-[2.5px]"
        >
          {#if battery}
            <span
              class="h-[8px] rounded-[1.5px] bg-current transition-[width] duration-500"
              style:width="max(2px, {Math.min(100, battery.percent)}%)"
            ></span>
          {/if}
          <span
            class="absolute top-1/2 -right-[4px] h-[6px] w-[2px] -translate-y-1/2 rounded-[1px] bg-current"
          ></span>
        </span>
      </span>

      {#if battery}
        <span class="flex flex-col gap-[3px] leading-none">
          <span class="flex items-center gap-1">
            <span class="text-[17px] font-semibold tabular-nums {batteryTone}">
              {battery.percent}%
            </span>
            {#if battery.charging}
              <Icon name="bolt" class="island-pulse size-3 text-[var(--di-green)]" />
            {/if}
          </span>
          <span class="text-[8px] font-semibold tracking-[0.09em] text-[var(--faint)] uppercase">
            {battery.charging
              ? 'Mengisi'
              : battery.percent <= 15
                ? 'Baterai lemah'
                : battery.percent <= 30
                  ? 'Baterai rendah'
                  : 'Baterai'}
          </span>
        </span>
      {:else}
        <span class="text-[12px] font-medium text-[var(--muted)]">Tanpa baterai</span>
      {/if}
    </span>
  {:else if variant === 'notification'}
    <span class="flex min-w-0 flex-1 items-center gap-2.5">
      <span
        class="flex size-7 shrink-0 items-center justify-center rounded-[9px] ring-1 transition-[background,box-shadow] duration-500 {currentNotice
          ? 'bg-[linear-gradient(145deg,color-mix(in_srgb,var(--di-purple)_34%,transparent),color-mix(in_srgb,var(--di-blue)_12%,transparent))] shadow-[0_0_12px_color-mix(in_srgb,var(--di-purple)_26%,transparent)] ring-[color:color-mix(in_srgb,var(--di-purple)_42%,transparent)]'
          : 'bg-[var(--chip)] ring-[color:var(--line)]'}"
      >
        <Icon
          name="bell"
          class="size-3.5 {currentNotice ? 'text-[var(--di-purple)]' : 'text-[var(--faint)]'}"
        />
      </span>

      {#if currentNotice}
        <span class="flex min-w-0 flex-1 flex-col gap-[4px] leading-none">
          <span class="truncate text-[13px] font-semibold">{currentNotice.title}</span>
          <span class="flex min-w-0 items-center gap-1.5 text-[10px]">
            <span class="max-w-[50%] shrink-0 truncate font-medium text-[var(--di-purple)]">
              {currentNotice.app}
            </span>
            {#if currentNotice.body}
              <span class="shrink-0 text-[var(--faint)]">·</span>
              <span class="min-w-0 truncate text-[var(--muted)]">{currentNotice.body}</span>
            {/if}
          </span>
          {#if notificationLog.length > 1}
            <span class="mt-[1px] flex h-[2px] w-full overflow-hidden rounded-full bg-[var(--track)]">
              {#key `${noticeIndex}-${currentNotice.title}`}
                <span
                  class="notif-progress h-full rounded-full bg-[var(--di-purple)]"
                ></span>
              {/key}
            </span>
          {/if}
        </span>
      {:else}
        <span class="flex min-w-0 flex-col gap-[3px] leading-none">
          <span class="text-[13px] leading-none font-semibold">Tidak ada notifikasi</span>
          <span class="truncate text-[10px] text-[var(--muted)]">
            {island.notifications
              ? 'Notifikasi Windows akan tampil di sini'
              : 'Aktifkan Notifikasi Windows di Setelan'}
          </span>
        </span>
      {/if}
    </span>
  {:else if variant === 'music'}
    <span class="flex min-w-0 flex-1 items-center gap-2.5">
      <span
        class="relative flex size-7 shrink-0 items-center justify-center overflow-hidden rounded-[9px] ring-1 transition-[background,box-shadow] duration-500 {playing
          ? 'bg-[linear-gradient(145deg,color-mix(in_srgb,var(--di-purple)_38%,transparent),color-mix(in_srgb,var(--di-blue)_14%,transparent))] shadow-[0_0_12px_color-mix(in_srgb,var(--di-purple)_30%,transparent)] ring-[color:color-mix(in_srgb,var(--di-purple)_45%,transparent)]'
          : 'bg-[var(--chip)] ring-[color:var(--line)]'}"
      >
        {#if playing}
          <span class="flex h-3.5 items-end gap-[2px] text-[var(--di-purple)]">
            {#each [0, 1, 2] as bar}
              <span
                class="eq-bar w-[2.5px] rounded-full bg-current"
                style:height="{7 + bar * 2}px"
                style:animation-delay="{bar * 130}ms"
              ></span>
            {/each}
          </span>
        {:else}
          <Icon
            name="media"
            class="size-3.5 {media ? 'text-[var(--di-purple)]' : 'text-[var(--faint)]'}"
          />
        {/if}
      </span>

      <span class="flex min-w-0 flex-1 flex-col gap-[4px] leading-none">
        <span class="truncate text-[13px] font-semibold">
          {media?.title || 'Belum ada media'}
        </span>
        <span class="flex min-w-0 items-center gap-1.5 text-[10px]">
          {#if media}
            {#if media.artist}
              <span class="min-w-0 truncate text-[var(--muted)]">{media.artist}</span>
              {#if media.app}
                <span class="shrink-0 text-[var(--faint)]">·</span>
              {/if}
            {/if}
            {#if media.app}
              <span
                class="max-w-[40%] shrink-0 truncate {media.artist
                  ? 'text-[var(--faint)]'
                  : 'text-[var(--muted)]'}"
              >
                {media.app}
              </span>
            {/if}
          {:else}
            <span class="truncate text-[var(--muted)]">Putar musik untuk mengontrolnya</span>
          {/if}
        </span>
        {#if media}
          <span class="mt-[1px] flex h-[2px] w-full overflow-hidden rounded-full bg-[var(--track)]">
            <span
              class="h-full rounded-full bg-[var(--di-purple)] transition-[width] duration-500"
              style:width="{musicProgress}%"
            ></span>
          </span>
        {/if}
      </span>
    </span>
  {:else if variant === 'mon'}
    <span class="flex min-w-0 flex-1 items-center gap-2.5">
      <span
        class="flex size-7 shrink-0 items-center justify-center rounded-[9px] bg-[color:color-mix(in_srgb,var(--di-blue)_16%,transparent)] ring-1 ring-[color:color-mix(in_srgb,var(--di-blue)_30%,transparent)]"
      >
        <Icon name="activity" class="size-3.5 text-[var(--di-blue)]" />
      </span>
      <span class="flex min-w-0 flex-1 items-center gap-3">
        {#each [
          { label: 'CPU', value: stats?.cpu ?? 0, color: 'var(--di-red)' },
          { label: 'RAM', value: stats?.ram ?? 0, color: 'var(--di-purple)' },
          { label: 'JAR', value: netLoad, color: 'var(--di-blue)' },
        ] as group}
          <span class="flex min-w-0 flex-1 flex-col gap-[3px]">
            <span class="flex items-baseline justify-between gap-1 leading-none">
              <span class="text-[8px] font-medium tracking-[0.08em] text-[var(--faint)] uppercase">
                {group.label}
              </span>
              <span class="text-[10px] font-semibold tabular-nums" style:color={group.color}>
                {Math.round(group.value)}
              </span>
            </span>
            <span class="relative h-[3px] w-full overflow-hidden rounded-full bg-[var(--track)]">
              <span
                class="absolute inset-y-0 left-0 rounded-full transition-[width] duration-500"
                style:width="{Math.max(3, Math.min(100, group.value))}%"
                style:background={group.color}
              ></span>
            </span>
          </span>
        {/each}
      </span>
    </span>
  {:else if variant === 'timer'}
    <span class="flex flex-1 items-center justify-center gap-2.5">
      <span class="relative flex size-[34px] shrink-0 items-center justify-center">
        <svg class="absolute inset-0 size-[34px] -rotate-90" viewBox="0 0 34 34" fill="none">
          <circle cx="17" cy="17" r="14.5" stroke="var(--track)" stroke-width="2.5" />
          <circle
            cx="17"
            cy="17"
            r="14.5"
            stroke={timerAccent}
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-dasharray={ring}
            stroke-dashoffset={ringOffset}
          />
        </svg>
        <Icon
          name="timer"
          class="size-4"
          style="color: {timerAccent}"
        />
      </span>
      <span class="flex min-w-0 flex-col gap-[3px] leading-none">
        <span class="text-[17px] font-semibold tabular-nums">
          {timerParts[0]}<span style="color: {timerAccent}">.</span>{timerParts[1]}
        </span>
        <span
          class="max-w-[74px] truncate text-[8px] font-semibold tracking-[0.09em] text-[var(--faint)] uppercase"
          >{timerCaption}</span
        >
      </span>
    </span>
  {/if}
</div>

<style>
  /* Island.svelte keys this view by variant, so the animation restarts on every switch:
     the content glides in from the right while the pill transitions its width over the
     same `--morph-ms`. Clipped by the pill, so it reads as sliding inside it. */
  .idle-swap {
    animation: idle-swap-in var(--morph-ms, 200ms) var(--ease-out) both;
  }

  @keyframes idle-swap-in {
    from {
      opacity: 0;
      transform: translateX(14px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .idle-swap {
      animation: none;
    }
  }
</style>
