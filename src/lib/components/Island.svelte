<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { island, type PanelTab } from '$lib/stores/island.svelte';
  import {
    cursorProbe,
    fitIslandWindow,
    setIslandPassthrough,
    setIslandHoverRegion,
    startCursorMonitor,
    usesNativeRuntime,
  } from '$lib/services/desktop';
  import type { CursorProbe, IdleVariant } from '$lib/types/system';
  import { parseGreetingPool, type Greeting } from '$lib/greetings';
  import IdleView from './IdleView.svelte';
  import WelcomeView from './WelcomeView.svelte';
  import HudView from './HudView.svelte';
  import NoticeView from './NoticeView.svelte';
  import SystemPanel from './SystemPanel.svelte';
  import { PANEL_HEIGHTS, PANEL_MIN_HEIGHT } from '$lib/panel-metrics';

  const IDLE_SIZES: Record<IdleVariant, { width: number; height: number }> = {
    eyes: { width: 282, height: 50 },
    min: { width: 148, height: 50 },
    music: { width: 264, height: 50 },
    mon: { width: 260, height: 50 },
    timer: { width: 156, height: 50 },
    battery: { width: 156, height: 50 },
    notification: { width: 320, height: 50 },
  };
  // Only these two put the scalable mascot inline with text, so their width has to grow
  // with it. Base = the row without the badge, plus the 34px base it scales from.
  const MASCOT_ROW_WIDTH: Partial<Record<IdleVariant, number>> = { eyes: 248, min: 114 };
  const NOTICE_SIZE = { width: 320, height: 50 };
  // The OS window is sized to this maximum. In rail layout the vertical menu is the
  // tallest element (header 48 + 6 + rail 8×32+7×4+8 + bottom pad 12 = 358), so the
  // expanded height matches it and the rail never clips out of the body. Dropping a tab
  // means dropping 36 px here, otherwise the panel keeps a dead strip under the menu.
  const EXPANDED_SIZE = { width: 672, height: 358 };

  // Horizontal layout only: each tab paints a tighter box, so the panel shrinks to the
  // content and the trimmed strip below never traps desktop clicks. In rail layout the
  // menu is the same height on every tab, so the island stays at EXPANDED_SIZE.height.
  function panelHeight(tab: PanelTab): number {
    if (island.navLayout === 'rail') return EXPANDED_SIZE.height;
    const wanted = PANEL_HEIGHTS[tab] ?? EXPANDED_SIZE.height;
    return Math.min(EXPANDED_SIZE.height, Math.max(PANEL_MIN_HEIGHT, wanted));
  }
  const HUD_MIN_WIDTH = 240;
  const WELCOME_SIZE = { width: 288, height: 50 };

  let welcoming = $state(true);
  let welcomeSeq = $state(0);
  // Non-empty means the current greeting draws from the reappearance pool instead of the
  // startup greeting. Assigned right before `welcomeSeq` bumps, which remounts WelcomeView.
  let welcomeEntries = $state<Greeting[]>([]);
  // Advances only on a real reappearance, so editing the pool in Settings previews one
  // stable line instead of cycling while you type.
  let welcomeIndex = $state(0);
  let welcomeTimer: ReturnType<typeof setTimeout> | undefined;
  let welcomeStartedAt = Date.now();
  let welcomeRemaining = 6000;
  let welcomePaused = false;

  const idleSize = $derived.by(() => {
    const base = IDLE_SIZES[island.activeIdle];
    const row = MASCOT_ROW_WIDTH[island.activeIdle];
    if (row === undefined) return base;
    return { width: Math.round(row + 34 * island.mascotScale), height: base.height };
  });
  const idleVisibleSize = $derived(welcoming ? WELCOME_SIZE : idleSize);
  const hudSize = $derived({
    width: Math.max(idleVisibleSize.width, HUD_MIN_WIDTH),
    height: idleVisibleSize.height,
  });
  const box = $derived(
    island.mode === 'panel'
      ? {
          width: EXPANDED_SIZE.width,
          height: panelHeight(island.panelTab),
          radius: island.panelRadius,
        }
      : island.mode === 'notice'
        ? { ...NOTICE_SIZE, radius: island.idleRadius }
        : island.mode === 'hud'
          ? { ...hudSize, radius: island.idleRadius }
          : { ...idleVisibleSize, radius: island.idleRadius },
  );

  // Inside Tauri, hover is delivered by the native cursor monitor (no polling). This
  // interval is only the plain-browser dev fallback.
  const POLL_MS = 24;
  const IDLE_DIM_MS = 30_000;
  const PANEL_HYSTERESIS = 4;
  const AUTO_IDLE_MS = 6000;

  let poll: ReturnType<typeof setInterval> | undefined;
  let unlistenToggle: (() => void) | undefined;
  let unlistenFullscreen: (() => void) | undefined;
  let unlistenShown: (() => void) | undefined;
  let unlistenCursor: (() => void) | undefined;
  let unlistenKey: (() => void) | undefined;
  let outsideSince = 0;
  let suppressHoverUntil = 0;
  let disposed = false;
  let passthrough = true;
  let cursorOver = false;
  let dimmed = $state(false);
  let peek = $state(false);
  let panelMounted = $state(false);
  let panelCloseTimer: ReturnType<typeof setTimeout> | undefined;

  async function applyPassthrough(ignore: boolean) {
    if (passthrough === ignore) return;
    passthrough = ignore;
    await setIslandPassthrough(ignore);
  }

  function regionFor(mode: string) {
    if (mode === 'panel')
      return {
        width: EXPANDED_SIZE.width + PANEL_HYSTERESIS * 2,
        height: panelHeight(island.panelTab) + PANEL_HYSTERESIS,
      };
    if (mode === 'notice') return NOTICE_SIZE;
    if (mode === 'hud') return hudSize;
    return idleVisibleSize;
  }

  function pushRegion(mode: string) {
    const r = regionFor(mode);
    void setIslandHoverRegion(r.width, r.height);
  }

  function scheduleWelcome() {
    clearTimeout(welcomeTimer);
    welcomeStartedAt = Date.now();
    welcomePaused = false;
    welcomeTimer = setTimeout(endWelcome, welcomeRemaining);
  }

  // Re-run the greeting overlay from a fresh full duration. `entries` makes it a pool
  // greeting (which line is picked from `welcomeIndex`); leaving it empty replays the
  // startup greeting — used by the Settings preview and the duration slider.
  function replayWelcome(entries: Greeting[] = []) {
    welcomeEntries = entries;
    welcomeRemaining = island.welcomeMs;
    welcomeSeq += 1;
    welcoming = true;
    dimmed = false;
    scheduleWelcome();
    pushRegion('idle');
  }

  // The island is only ever hidden two ways: the fullscreen watcher covers the whole
  // screen with it, and the tray/hotkey hides the window. Both land here so coming back
  // from either greets again — with a different line when a pool is configured. A pill
  // that is mid-overlay or mid-panel keeps showing that instead.
  function greetAfterHiding() {
    if (disposed || island.fullscreen || island.mode !== 'idle') return;
    const pool = parseGreetingPool(island.greetingPool);
    replayWelcome(pool);
    // The next return from hiding plays the following line, never the one just shown.
    if (pool.length > 1) welcomeIndex = (welcomeIndex + 1) % pool.length;
  }

  function pauseWelcome() {
    if (!welcoming || welcomePaused) return;
    welcomePaused = true;
    clearTimeout(welcomeTimer);
    welcomeRemaining = Math.max(0, welcomeRemaining - (Date.now() - welcomeStartedAt));
  }

  function resumeWelcome() {
    if (!welcoming || !welcomePaused) return;
    scheduleWelcome();
  }

  function endWelcome() {
    if (!welcoming) return;
    welcoming = false;
    welcomePaused = false;
    clearTimeout(welcomeTimer);
    pushRegion(island.mode);
  }

  async function openPanel(tab: PanelTab = island.contentTab) {
    if (island.mode === 'panel') return;
    endWelcome();
    panelMounted = true;
    island.pinned = false;
    island.panelTab = tab;
    island.mode = 'panel';
    suppressHoverUntil = 0;
    pushRegion('panel');
    await applyPassthrough(false);
  }

  async function closePanel() {
    island.pinned = false;
    peek = false;
    island.mode = island.overlayMode;
    suppressHoverUntil = Date.now() + 150;
    pushRegion(island.overlayMode);
    await applyPassthrough(true);
  }

  // HUD and notice deadlines must also collapse the pill when the mouse is not moving;
  // the native cursor monitor only emits on motion or hover-edge changes.
  function syncOverlay() {
    if (disposed || island.fullscreen) return;
    if (island.mode !== 'hud' && island.mode !== 'notice') return;
    const wanted = island.overlayMode;
    if (wanted === island.mode) return;
    island.mode = wanted;
    pushRegion(wanted);
    void applyPassthrough(!cursorOver);
  }

  async function togglePinned() {
    endWelcome();
    if (island.pinned) {
      island.pinned = false;
      peek = false;
      if (island.mode === 'panel') {
        island.mode = island.overlayMode;
        suppressHoverUntil = Date.now() + 150;
        pushRegion(island.overlayMode);
        await applyPassthrough(true);
      }
      return;
    }

    island.pinned = true;
    outsideSince = 0;
    if (island.fullscreen) peek = true;
    panelMounted = true;
    island.mode = 'panel';
    pushRegion('panel');
    await applyPassthrough(false);
  }

  function onCursor(p: CursorProbe) {
    if (disposed) return;
    cursorOver = p.over;

    if (island.fullscreen && !peek) {
      if (island.mode !== 'idle') {
        island.mode = 'idle';
        pushRegion('idle');
        void applyPassthrough(true);
      }
      return;
    }

    if (island.mode === 'idle') island.cursor = { x: p.dx, y: p.dy };

    if (p.over && welcoming) {
      endWelcome();
    }

    const now = Date.now();
    if (p.over) {
      outsideSince = 0;
      dimmed = false;
    } else {
      if (!outsideSince) outsideSince = now;
      if (island.mode === 'idle' && !welcoming && now - outsideSince >= IDLE_DIM_MS)
        dimmed = true;
    }

    let wanted = island.mode;
    if (p.over && now >= suppressHoverUntil && island.mode !== 'panel') {
      // Landing on the subject the pill is currently about (download, media, notice…),
      // not always on the first page.
      island.panelTab = island.contentTab;
      wanted = 'panel';
    } else if (island.mode === 'panel') {
      if (!island.pinned && !p.over) wanted = island.overlayMode;
    } else if (
      (island.mode === 'notice' || island.mode === 'hud') &&
      island.overlayMode !== island.mode
    ) {
      wanted = island.overlayMode;
    }

    if (wanted !== island.mode) {
      if (wanted === 'panel') endWelcome();
      island.mode = wanted;
      pushRegion(wanted);
    }

    void applyPassthrough(!(wanted === 'panel' || p.over));
  }

  // If the active variant changes (e.g. chosen from Settings), immediately dismiss
  // welcome so the user sees the new variant right away without any delay. The first
  // effect run only records the initial variant; ending welcome there would hide it
  // before it is ever painted.
  let lastActiveIdle: IdleVariant | undefined;
  $effect(() => {
    const active = island.activeIdle;
    if (lastActiveIdle !== undefined && active !== lastActiveIdle && welcoming) endWelcome();
    lastActiveIdle = active;
  });

  // Changing the greeting in Settings should be immediately visible: replay the overlay
  // on the idle pill. Skip the first run and any moment the pill is not idle, so an open
  // panel or a live HUD/notice is never hijacked by the preview.
  let lastGreeting: string | undefined;
  let lastPool: string | undefined;
  $effect(() => {
    const signature = `${island.greetingMode}::${island.greetingText}::${island.greetingText2}::${island.greetingShowLine2}`;
    const pool = island.greetingPool;
    const mainChanged = lastGreeting !== undefined && lastGreeting !== signature;
    const poolChanged = lastPool !== undefined && lastPool !== pool;
    lastGreeting = signature;
    lastPool = pool;
    if (!mainChanged && !poolChanged) return;
    if (island.mode !== 'idle' || island.fullscreen) return;
    // Editing the main greeting previews that text; editing only the pool previews a line
    // from it, so the two inputs never show the same thing.
    replayWelcome(mainChanged ? [] : parseGreetingPool(pool));
  });

  // `welcome_ms` is its own timing, separate from the variant morph. The store arrives
  // after mount (settings load is async), so re-arm a running greeting when it lands or
  // when the slider moves — the remount also rescales the progress ring.
  let lastWelcomeMs: number | undefined;
  $effect(() => {
    const ms = island.welcomeMs;
    const changed = lastWelcomeMs !== undefined && lastWelcomeMs !== ms;
    lastWelcomeMs = ms;
    if (changed && welcoming && island.mode === 'idle' && !island.fullscreen) replayWelcome();
  });

  // The panel is by far the heaviest subtree (all five tabs, sparklines, live metric
  // transitions). Keep it mounted only while it is open or fading out; an invisible
  // panel otherwise keeps repainting and animating behind the pill.
  $effect(() => {
    if (island.mode === 'panel') {
      clearTimeout(panelCloseTimer);
      panelMounted = true;
      return;
    }
    if (!panelMounted) return;
    clearTimeout(panelCloseTimer);
    panelCloseTimer = setTimeout(() => (panelMounted = false), 220);
  });

  // Keep the native hover rectangle in sync with the rendered pill. Overlays are also
  // opened from store events (volume, downloads, notifications), so relying only on the
  // explicit push calls in open/close would leave the monitor testing the stale size.
  $effect(() => {
    const mode = island.mode;
    if (mode === 'idle') {
      void setIslandHoverRegion(idleVisibleSize.width, idleVisibleSize.height);
    } else if (mode === 'notice' || mode === 'hud') {
      pushRegion(mode);
    }
  });

  // Each panel tab paints a different box height, so both the hover rectangle and the
  // native window must follow the active tab. Without the resize the trimmed strip below
  // a short panel would stay part of the (click-capturing) window and trap desktop clicks.
  // Width is constant and the window is top-anchored, so the height change never nudges
  // it horizontally. The box animates its height over 200ms, so when shrinking we hold
  // the window at the old (larger) height until the animation lands, otherwise the glow's
  // rounded bottom corner gets clipped by the window edge mid-transition.
  let lastPanelHeight = EXPANDED_SIZE.height;
  let panelResizeTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (island.mode !== 'panel') {
      lastPanelHeight = EXPANDED_SIZE.height;
      return;
    }
    const height = panelHeight(island.panelTab);
    pushRegion('panel');
    const larger = Math.max(height, lastPanelHeight);
    void fitIslandWindow(EXPANDED_SIZE.width, larger, island.position);
    clearTimeout(panelResizeTimer);
    if (larger !== height) {
      panelResizeTimer = setTimeout(() => {
        void fitIslandWindow(EXPANDED_SIZE.width, height, island.position);
      }, 220);
    }
    lastPanelHeight = height;
    return () => clearTimeout(panelResizeTimer);
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && island.mode === 'panel') {
      event.preventDefault();
      void closePanel();
    }
  }

  // Reactive timers: an always-running interval stops the renderer from sleeping
  // between updates, so each one ticks only while its view is on screen.
  $effect(() => {
    if (island.timer?.status !== 'running') return;
    // The countdown is derived from the deadline Rust owns, so this only advances the
    // clock the pill reads; 250 ms keeps the printed second honest without a repaint
    // per frame.
    const tick = setInterval(() => (island.now = Date.now()), 250);
    return () => clearInterval(tick);
  });
  $effect(() => {
    const mode = island.mode;
    if (
      island.idleStyle !== 'auto' ||
      mode !== 'idle' ||
      island.fullscreen ||
      welcoming
    ) {
      return;
    }
    const cycle = setInterval(() => island.cycleIdle(), AUTO_IDLE_MS);
    return () => clearInterval(cycle);
  });
  $effect(() => {
    const mode = island.mode;
    if (mode !== 'hud' && mode !== 'notice') return;
    const overlay = setInterval(syncOverlay, 150);
    return () => clearInterval(overlay);
  });

  onMount(() => {
    void listen('toggle-panel', () => void togglePinned()).then((off) => {
      unlistenToggle = off;
    });
    // The pill opens on hover, so the webview usually has no keyboard focus and its
    // keydown handler never sees Escape. The global hook still reports it; use that
    // to dismiss the panel instead of flashing a key HUD.
    void listen<string>('key-hud', (event) => {
      if (island.mode === 'panel') {
        if (event.payload === 'Esc') void closePanel();
        return;
      }
      island.flash({ kind: 'key', key: event.payload });
    }).then((off) => {
      unlistenKey = off;
    });
    void listen<boolean>('fullscreen-changed', (event) => {
      island.fullscreen = event.payload;
      peek = false;
      endWelcome();
      if (event.payload) {
        island.pinned = false;
        island.mode = 'idle';
        void applyPassthrough(true);
      } else {
        // Cursor tracking resumes now; if the pointer happens to rest on the pill,
        // do not pop the panel open until the user actually moves it.
        cursorOver = false;
        suppressHoverUntil = Date.now() + 500;
        greetAfterHiding();
      }
    }).then((off) => {
      unlistenFullscreen = off;
    });
    // Tray "Sembunyikan/Tampilkan island" and the reveal hotkey never reach the DOM as a
    // visibility change, so the native side announces the window reappearing.
    void listen('island-shown', () => greetAfterHiding()).then((off) => {
      unlistenShown = off;
    });

    scheduleWelcome();

    void (async () => {
      await fitIslandWindow(EXPANDED_SIZE.width, EXPANDED_SIZE.height, island.position);
      passthrough = true;
      await setIslandPassthrough(true);
      pushRegion(island.mode);
      if (usesNativeRuntime()) {
        await startCursorMonitor();
        unlistenCursor = await listen<CursorProbe>('island-cursor', (event) =>
          onCursor(event.payload),
        );
      } else {
        poll = setInterval(() => {
          const r = regionFor(island.mode);
          void cursorProbe(r.width, r.height).then(onCursor);
        }, POLL_MS);
      }
    })();

    return () => {
      disposed = true;
      clearInterval(poll);
      clearTimeout(welcomeTimer);
      clearTimeout(panelCloseTimer);
      unlistenToggle?.();
      unlistenFullscreen?.();
      unlistenShown?.();
      unlistenCursor?.();
      unlistenKey?.();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="island-glow relative overflow-hidden text-[var(--ink)]"
  class:island-dim={island.mode === 'idle' && dimmed}
  class:island-hidden={island.fullscreen && !peek}
  class:island-welcome={welcoming && island.mode === 'idle'}
  data-tab={island.mode === 'panel' ? island.panelTab : 'system'}
  data-morph={island.mode === 'panel' ? 'panel' : 'idle'}
  style:--morph-ms="{island.morphMs}ms"
  style:width="{box.width}px"
  style:height="{box.height}px"
  style:border-radius="{box.radius}px"
>
  <div
    class="island-view absolute top-0 left-1/2 -translate-x-1/2 {island.mode === 'idle'
      ? 'translate-y-0 scale-100 opacity-100'
      : 'translate-y-[6px] scale-[0.82] opacity-0'}"
    style:width="{idleVisibleSize.width}px"
    style:height="{idleVisibleSize.height}px"
    role="button"
    tabindex={island.mode === 'idle' ? 0 : -1}
    aria-label="Buka panel Desk Island"
    inert={island.mode !== 'idle'}
    onclick={() => void openPanel()}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        void openPanel();
      }
    }}
  >
    {#if welcoming}
      {#key welcomeSeq}
        <WelcomeView
          duration={island.welcomeMs}
          index={welcomeIndex}
          entries={welcomeEntries}
        />
      {/key}
    {:else}
      {#key island.activeIdle}
        <IdleView />
      {/key}
    {/if}
  </div>

  <div
    class="island-view absolute top-0 left-1/2 -translate-x-1/2 {island.mode === 'notice'
      ? 'translate-y-0 scale-100 opacity-100'
      : 'translate-y-[6px] scale-[0.82] opacity-0'}"
    style:width="{NOTICE_SIZE.width}px"
    style:height="{NOTICE_SIZE.height}px"
    role="button"
    tabindex={island.mode === 'notice' ? 0 : -1}
    aria-label="Buka panel Desk Island"
    inert={island.mode !== 'notice'}
    onclick={() => void openPanel()}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        void openPanel();
      }
    }}
  >
    <NoticeView />
  </div>

  <div
    class="island-view absolute top-0 left-1/2 -translate-x-1/2 {island.mode === 'hud'
      ? 'translate-y-0 scale-100 opacity-100'
      : 'translate-y-[6px] scale-[0.82] opacity-0'}"
    style:width="{hudSize.width}px"
    style:height="{hudSize.height}px"
    role="button"
    tabindex={island.mode === 'hud' ? 0 : -1}
    aria-label="Buka panel Desk Island"
    inert={island.mode !== 'hud'}
    onclick={() => void openPanel()}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        void openPanel();
      }
    }}
  >
    <HudView />
  </div>

  <div
    class="island-view absolute inset-0 {island.mode === 'panel'
      ? 'translate-y-0 scale-100 opacity-100'
      : 'pointer-events-none translate-y-[6px] scale-[0.82] opacity-0'}"
    inert={island.mode !== 'panel'}
  >
    {#if panelMounted}
      <SystemPanel />
    {/if}
  </div>
</div>
