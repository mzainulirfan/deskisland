<script lang="ts">
  import { untrack } from 'svelte';
  import Eyes from './Eyes.svelte';
  import Icon, { type IconName } from './Icon.svelte';
  import MetricCard from './MetricCard.svelte';
  import MediaPanel from './MediaPanel.svelte';
  import ProcessPanel from './ProcessPanel.svelte';
  import QuickSettingsPanel from './QuickSettingsPanel.svelte';
  import RemindersPanel from './RemindersPanel.svelte';
  import StatTile from './StatTile.svelte';
  import TimerPanel from './TimerPanel.svelte';
  import VolumePanel from './VolumePanel.svelte';
  import DownloadsPanel from './DownloadsPanel.svelte';
  import AppsPanel from './AppsPanel.svelte';
  import { openSettingsWindow } from '$lib/services/desktop';
  import { island, type PanelTab } from '$lib/stores/island.svelte';
  import { paneHeight } from '$lib/panel-metrics';

  // Media and system volume share one page: both are short cards, and the pill's music /
  // volume pills then have a single subject to open. `full` labels the tooltip because the
  // tab strip clips to 60 px.
  const tabs: { id: PanelTab; label: string; full?: string; icon: IconName }[] = [
    { id: 'system', label: 'Sistem', icon: 'activity' },
    { id: 'media', label: 'Media', full: 'Media & Volume', icon: 'media' },
    { id: 'timer', label: 'Timer', full: 'Timer & Pomodoro', icon: 'timer' },
    { id: 'reminders', label: 'Pengingat', full: 'Pengingat Harian', icon: 'bell' },
    { id: 'processes', label: 'Proses', icon: 'cpu' },
    { id: 'downloads', label: 'Unduhan', icon: 'download' },
    { id: 'apps', label: 'Aplikasi', icon: 'apps' },
    { id: 'quick', label: 'Cepat', icon: 'sliders' },
  ];

  const horizontal = $derived(island.navLayout === 'tabs');

  const stats = $derived(island.stats);
  const battery = $derived(stats?.battery);
  const networkRate = $derived((stats?.rx ?? 0) + (stats?.tx ?? 0));
  const dateShort = $derived(
    island.time
      ? new Intl.DateTimeFormat('id-ID', { weekday: 'short', day: 'numeric', month: 'short' }).format(
          new Date(),
        )
      : '',
  );
  const batteryTone = $derived(
    battery == null
      ? 'text-[var(--muted)]'
      : battery.charging || battery.percent > 30
        ? 'text-[var(--di-green)]'
        : battery.percent > 15
          ? 'text-[var(--di-amber)]'
          : 'text-[var(--di-red)]',
  );

  const gib = 1024 ** 3;
  const gb = (value: number) => `${(value / gib).toFixed(1).replace('.', ',')} GB`;

  function rate(bytesPerSecond: number) {
    if (bytesPerSecond >= 1024 ** 2) return `${(bytesPerSecond / 1024 ** 2).toFixed(1)} MB/s`;
    if (bytesPerSecond >= 1024) return `${Math.round(bytesPerSecond / 1024)} KB/s`;
    return `${Math.round(bytesPerSecond)} B/s`;
  }

  let cpuHistory = $state<number[]>([]);
  let ramHistory = $state<number[]>([]);
  let networkHistory = $state<number[]>([]);

  $effect(() => {
    const next = island.stats;
    if (!next) return;
    untrack(() => {
      cpuHistory = [...cpuHistory, next.cpu].slice(-36);
      ramHistory = [...ramHistory, next.ram].slice(-36);
      networkHistory = [...networkHistory, next.rx + next.tx].slice(-36);
    });
  });

  const networkMax = $derived(Math.max(100 * 1024, ...networkHistory));

  // Horizontal layout only: the pane track slides and can be dragged between tabs.
  const SWIPE_THRESHOLD = 36;
  const DRAG_START = 6;
  const EDGE_RESISTANCE = 0.3;

  let dragOffset = $state(0);
  let dragging = $state(false);
  let dragged = false;
  let startX = 0;
  let startIndex = 0;

  const tabIndex = $derived(Math.max(0, tabs.findIndex((tab) => tab.id === island.panelTab)));
  const paneWidth = $derived(100 / tabs.length);
  const trackShift = $derived(-tabIndex * paneWidth);
  // Hidden panes would still re-render and poll every stats tick while parked in the
  // sliding track. Mount the active pane plus its swipe neighbours so a drag shows
  // content, and let the far panes unmount entirely.
  const mounted = (index: number) => Math.abs(tabIndex - index) <= 1;

  // The menu opens a page on hover, not only on click. The short intent delay is the same
  // trick the pill uses to expand: sweeping the cursor across the rail must not flip
  // through every tab on the way to the one that was meant.
  const HOVER_TAB_MS = 120;
  // Activating a tab in the horizontal layout grows its button (the label fades in), which
  // slides every button after it. Chromium then reports the button that moved under a
  // motionless cursor as a fresh hover, and the page walks away from the pointer one tab at
  // a time. Ignoring entries for a moment after a switch lets the strip settle first.
  const HOVER_SETTLE_MS = 350;
  let settledAt = 0;
  let pendingTab: PanelTab | undefined;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  function hoverTab(tab: PanelTab) {
    if (island.panelTab === tab) {
      cancelHoverTab();
      return;
    }
    if (Date.now() - settledAt < HOVER_SETTLE_MS) return;
    if (pendingTab === tab) return;
    pendingTab = tab;
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => {
      pendingTab = undefined;
      settledAt = Date.now();
      island.panelTab = tab;
    }, HOVER_TAB_MS);
  }

  function cancelHoverTab() {
    clearTimeout(hoverTimer);
    pendingTab = undefined;
  }

  $effect(() => () => clearTimeout(hoverTimer));

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest('button, input, select, textarea, a, [role="slider"], [data-no-swipe]')) {
      return;
    }
    dragging = true;
    dragged = false;
    startX = event.clientX;
    startIndex = tabIndex;
    dragOffset = 0;
    try {
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {}
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging) return;
    const delta = event.clientX - startX;
    if (!dragged && Math.abs(delta) > DRAG_START) dragged = true;
    if (!dragged) return;
    const atStart = startIndex === 0 && delta > 0;
    const atEnd = startIndex === tabs.length - 1 && delta < 0;
    dragOffset = atStart || atEnd ? delta * EDGE_RESISTANCE : delta;
  }

  function endDrag(event: PointerEvent) {
    if (!dragging) return;
    const delta = dragOffset;
    if (dragged) {
      if (delta < -SWIPE_THRESHOLD && tabIndex < tabs.length - 1) {
        island.panelTab = tabs[tabIndex + 1].id;
      } else if (delta > SWIPE_THRESHOLD && tabIndex > 0) {
        island.panelTab = tabs[tabIndex - 1].id;
      }
    }
    dragging = false;
    dragOffset = 0;
    dragged = false;
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    } catch {}
  }
</script>

{#snippet pane(tab: PanelTab)}
  {#if tab === 'system'}
    <!-- Two load cards that own the spare height, plus a narrow column for the
         throughput/consumption readings that only need one line each. -->
    <div
      class="grid h-full min-h-0 grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,0.92fr)] gap-2"
      aria-label="Metrik sistem"
    >
      <MetricCard
        label="CPU"
        value={stats ? `${Math.round(stats.cpu)}%` : '—'}
        sub="Beban semua inti"
        color="var(--di-red)"
        history={cpuHistory}
      />
      <MetricCard
        label="RAM"
        value={stats ? `${Math.round(stats.ram)}%` : '—'}
        sub={stats ? `${gb(stats.ramUsed)} / ${gb(stats.ramTotal)}` : 'Memori terpakai'}
        color="var(--di-purple)"
        history={ramHistory}
      />
      <div class="flex min-h-0 min-w-0 flex-col gap-2">
        <StatTile
          label="Jaringan"
          icon="wifi"
          color="var(--di-blue)"
          value={stats ? rate(stats.rx) : '—'}
          sub={stats ? `unggah ${rate(stats.tx)}` : 'Lalu lintas masuk'}
          history={networkHistory}
          max={networkMax}
          tone="text-[var(--di-blue)]"
        />
        <StatTile
          label="Penyimpanan"
          icon="disk"
          color="var(--di-amber)"
          value={stats ? `${Math.round(stats.disk)}%` : '—'}
          sub={stats && stats.diskTotal > 0
            ? `${gb(stats.diskUsed)} / ${gb(stats.diskTotal)}`
            : 'Disk utama'}
          progress={stats?.disk ?? null}
          tone={stats && stats.disk > 85 ? 'text-[var(--di-red)]' : 'text-[var(--di-amber)]'}
        />
      </div>
    </div>
  {:else if tab === 'media'}
    <div
      class="audio-shell flex h-full min-h-0 items-stretch gap-3 rounded-[16px] p-2.5 ring-1 ring-[color:var(--line)]"
    >
      <div class="min-w-0 flex-1">
        <MediaPanel />
      </div>
      <span class="audio-sep" aria-hidden="true"></span>
      <div class="w-[216px] shrink-0">
        <VolumePanel />
      </div>
    </div>
  {:else if tab === 'timer'}
    <TimerPanel />
  {:else if tab === 'reminders'}
    <RemindersPanel />
  {:else if tab === 'processes'}
    <ProcessPanel />
  {:else if tab === 'downloads'}
    <DownloadsPanel />
  {:else if tab === 'apps'}
    <AppsPanel />
  {:else if tab === 'quick'}
    <QuickSettingsPanel />
  {/if}
{/snippet}

<section class="flex h-full w-full flex-col pb-3">
  <!-- Identity + status always sit on one row. In horizontal mode the tab strip is
       injected in the centre; in rail mode the centre stays empty and the strip lives
       in the vertical rail below. -->
  <div class="flex h-12 shrink-0 items-center gap-2 px-3.5">
    <div class="flex min-w-0 flex-1 basis-0 items-center gap-2.5">
      <span
        class="eyes-badge flex size-8 shrink-0 items-center justify-center rounded-[10px] bg-[linear-gradient(145deg,color-mix(in_srgb,var(--di-blue)_30%,transparent),color-mix(in_srgb,var(--di-purple)_14%,transparent))] ring-1 ring-[color:color-mix(in_srgb,var(--di-blue)_24%,transparent)]"
      >
        <Eyes />
      </span>
      <div class="flex min-w-0 flex-col leading-none">
        <span class="text-[15px] font-bold tracking-tight tabular-nums">{island.time}</span>
        <span
          class="mt-1 truncate text-[9px] font-medium uppercase tracking-[0.14em] text-[var(--faint)]"
          >{dateShort}</span
        >
      </div>
    </div>

    {#if horizontal}
      <div
        class="nav-bar flex shrink-0 items-center gap-0.5"
        role="tablist"
        tabindex="-1"
        aria-label="Bagian panel"
        onpointerleave={cancelHoverTab}
      >
        {#each tabs as tab (tab.id)}
          {@const active = island.panelTab === tab.id}
          <button
            type="button"
            class="tab-btn"
            class:tab-btn-on={active}
            role="tab"
            aria-selected={active}
            aria-label={tab.full ?? tab.label}
            title={tab.full ?? tab.label}
            onpointerenter={() => hoverTab(tab.id)}
            onclick={() => (island.panelTab = tab.id)}
          >
            <Icon name={tab.icon} class="size-4" />
            <span class="tab-label">{tab.label}</span>
          </button>
        {/each}
      </div>
    {/if}

    <div class="flex min-w-0 flex-1 basis-0 items-center justify-end gap-1.5">
      <span class="status-chip text-[var(--muted)]" title="Lalu lintas jaringan">
        <Icon name="wifi" class="size-3.5 text-[var(--di-blue)]" />
        <span>{rate(networkRate)}</span>
      </span>

      {#if battery}
        <span class="status-chip {batteryTone}" title={battery.charging ? 'Baterai sedang mengisi' : 'Baterai'}>
          <Icon name={battery.charging ? 'bolt' : 'battery'} class="size-3.5" />
          <span>{battery.percent}%</span>
        </span>
      {/if}

      <span class="mx-0.5 h-4 w-px shrink-0 bg-[var(--line)]" aria-hidden="true"></span>

      <button
        type="button"
        class="flex size-7 items-center justify-center rounded-full bg-[var(--chip)] text-[var(--muted)] ring-1 ring-[color:var(--line)] transition-colors hover:bg-[var(--di-surface-hover)] hover:text-[var(--ink)]"
        aria-label="Buka setelan"
        title="Buka setelan"
        onclick={() => void openSettingsWindow()}
      >
        <Icon name="settings" class="size-4" />
      </button>
    </div>
  </div>

  {#if horizontal}
    <div
      class="relative mt-3 min-h-0 flex-1 cursor-grab touch-pan-y overflow-hidden active:cursor-grabbing"
      role="group"
      aria-label="Isi bagian panel"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={endDrag}
      onpointercancel={endDrag}
    >
      <div
        class="flex h-full {dragging
          ? 'transition-none'
          : 'transition-transform duration-150 ease-[cubic-bezier(0.22,0.61,0.36,1)]'}"
        style:width="{tabs.length * 100}%"
        style:transform="translateX(calc({trackShift}% + {dragOffset}px))"
      >
        {#each tabs as tab, index (tab.id)}
          <div class="h-full shrink-0 px-3.5" style:width="{paneWidth}%" inert={tabIndex !== index}>
            {#if mounted(index)}
              <div class="h-full">{@render pane(tab.id)}</div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="mt-1.5 flex min-h-0 flex-1 gap-2 pl-2.5 pr-3.5">
      <div
        class="rail flex shrink-0 flex-col items-center gap-1 py-1"
        role="tablist"
        tabindex="-1"
        aria-label="Bagian panel"
        onpointerleave={cancelHoverTab}
      >
        {#each tabs as tab (tab.id)}
          {@const active = island.panelTab === tab.id}
          <button
            type="button"
            class="rail-btn"
            class:rail-btn-on={active}
            role="tab"
            aria-selected={active}
            aria-label={tab.full ?? tab.label}
            title={tab.full ?? tab.label}
            onpointerenter={() => hoverTab(tab.id)}
            onclick={() => (island.panelTab = tab.id)}
          >
            <Icon name={tab.icon} class="size-4" />
            <span class="rail-tip">{tab.full ?? tab.label}</span>
          </button>
        {/each}
      </div>

      <div class="relative min-w-0 flex-1 overflow-y-auto">
        {#key island.panelTab}
          <!-- The rail is the tall element of this layout, so the box stays at full height
               on every tab. The pane takes only the height its own tab asks for and sits at
               the top of the column: stretching it to the menu's height left most pages
               with a large empty interior, and the 4 px entry slide inside this
               `overflow-y-auto` column showed a scrollbar on every page change. -->
          <div class="pane" style:height="{paneHeight(island.panelTab)}px">
            {@render pane(island.panelTab)}
          </div>
        {/key}
      </div>
    </div>
  {/if}
</section>

<style>
  .status-chip {
    display: flex;
    height: 26px;
    flex-shrink: 0;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border-radius: 999px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 10.5px;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* Vertical rail (rail layout). */
  .rail {
    width: 44px;
  }

  .rail-btn {
    position: relative;
    display: flex;
    width: 36px;
    height: 32px;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    color: var(--muted);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .rail-btn:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--ink);
  }

  .rail-btn-on,
  .rail-btn-on:hover {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent-ink);
  }

  /* Floating label appears beside the icon without widening the fixed rail, so the
       pane never reflows when a name is shown. */
  .rail-tip {
    position: absolute;
    left: calc(100% + 6px);
    z-index: 20;
    pointer-events: none;
    border-radius: 7px;
    background: var(--card);
    padding: 2px 7px;
    color: var(--ink);
    box-shadow:
      inset 0 0 0 1px var(--line),
      0 4px 12px color-mix(in srgb, var(--ink) 18%, transparent);
    font-size: 10px;
    font-weight: 600;
    white-space: nowrap;
    opacity: 0;
    transform: translateX(-3px);
    transition:
      opacity 120ms var(--ease-out),
      transform 120ms var(--ease-out);
  }

  .rail-btn:hover .rail-tip {
    opacity: 1;
    transform: translateX(0);
  }

  /* Horizontal tab strip (tabs layout). */
  .tab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 30px;
    padding: 0 9px;
    border-radius: 9px;
    color: var(--muted);
    font-size: 11px;
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .tab-btn:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--ink);
  }

  .tab-btn-on,
  .tab-btn-on:hover {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent-ink);
  }

  .tab-label {
    max-width: 0;
    overflow: hidden;
    font-weight: 600;
    white-space: nowrap;
    transition: max-width 220ms var(--ease-out);
  }

  .tab-btn-on .tab-label {
    max-width: 60px;
  }

  /* Playback and volume share one surface: a single card split by a fading hairline
     instead of two stacked boxes, which frees half the pane's height. */
  .audio-shell {
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--di-purple) 7%, transparent),
        transparent 55%
      ),
      var(--card);
  }

  .audio-sep {
    width: 1px;
    flex-shrink: 0;
    background: linear-gradient(
      180deg,
      transparent,
      var(--line) 18%,
      var(--line) 82%,
      transparent
    );
  }

  .pane {
    animation: pane-in 160ms var(--ease-out);
  }

  @keyframes pane-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
