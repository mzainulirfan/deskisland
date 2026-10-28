<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import Island from '$lib/components/Island.svelte';
  import { island } from '$lib/stores/island.svelte';
  import {
    getDownloads,
    getNowPlaying,
    getRecentDownloads,
    getRecentNotifications,
    getReminders,
    getSystemStats,
    getTimer,
    getTopProcesses,
    getVolume,
    setIslandPosition,
  } from '$lib/services/desktop';
  import type { NotificationEvent, ReminderEvent, TimerInfo } from '$lib/types/system';
  import type { Settings } from '$lib/services/desktop';

  const formatTime = new Intl.DateTimeFormat('id-ID', {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });

  let busy = false;
  let volumeBusy = false;
  let mediaBusy = false;

  // Three chime notes beat a flat beep: nothing has to ship in the bundle and the alert
  // still carries a recognizable rhythm. The context is created on first use because a gesture
  // (starting the timer) has already unlocked audio by then.
  let audio: AudioContext | undefined;

  function chime() {
    const ctx = (audio ??= new AudioContext());
    if (ctx.state === 'suspended') void ctx.resume();
    const start = ctx.currentTime;
    [880, 1174.66, 1760].forEach((frequency, index) => {
      const oscillator = ctx.createOscillator();
      const gain = ctx.createGain();
      oscillator.type = 'sine';
      oscillator.frequency.value = frequency;
      const at = start + index * 0.16;
      gain.gain.setValueAtTime(0.0001, at);
      gain.gain.linearRampToValueAtTime(0.18, at + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.38);
      oscillator.connect(gain).connect(ctx.destination);
      oscillator.start(at);
      oscillator.stop(at + 0.4);
    });
  }

  // Media and processes only feed their own tab; polling them while another tab is
  // shown costs a COM round-trip and a full process enumeration every second for
  // nothing. This fires on panel open and on every tab switch.
  $effect(() => {
    if (!island.expanded) return;
    const tab = island.panelTab;
    if (tab === 'media') {
      void getNowPlaying().then((m) => {
        if (m) island.media = m;
      });
    }
    if (tab === 'processes') {
      void getTopProcesses(6).then((p) => {
        if (p && p.length) island.processes = p;
      });
    }
    if (tab === 'downloads') {
      void getRecentDownloads(6).then((l) => {
        island.downloadList = l;
      });
    }
  });

  onMount(() => {
    let unlistenKey: (() => void) | undefined;
    let unlistenNotification: (() => void) | undefined;
    let unlistenSettings: (() => void) | undefined;
    let unlistenTimer: (() => void) | undefined;
    let unlistenAlarm: (() => void) | undefined;
    let unlistenReminder: (() => void) | undefined;
    // The pill only needs CPU/RAM/net/battery and the download fingerprint. Media and
    // the process list are read on demand while the panel is open: they cost a COM
    // round-trip each and nobody sees them while the island is collapsed.
    const refresh = async () => {
      if (busy) return;
      busy = true;
      try {
        island.time = formatTime.format(new Date());
        island.stats = await getSystemStats();

        const downloads = await getDownloads();
        if (downloads) {
          island.downloads = downloads.count;
          if (downloads.event) island.announce({ kind: 'download', ...downloads.event });
        }

        if (island.expanded) {
          if (island.panelTab === 'media') island.media = await getNowPlaying();
          if (island.panelTab === 'processes') island.processes = await getTopProcesses(6);
          if (island.panelTab === 'downloads') island.downloadList = await getRecentDownloads(6);
        }
      } finally {
        busy = false;
      }
    };

    // Volume changes from media keys, the mixer or another app, so it is polled even
    // while the panel is open. Skipped during a drag so a slow read cannot overwrite
    // the optimistic value; the HUD flash only matters while the pill is collapsed.
    const watchVolume = async () => {
      if (volumeBusy || island.volumeEditing) return;
      volumeBusy = true;
      try {
        const next = await getVolume();
        if (!next || island.volumeEditing) return;
        const previous = island.volume;
        island.volume = next;
        if (
          !island.expanded &&
          previous &&
          (previous.percent !== next.percent || previous.muted !== next.muted)
        ) {
          island.flash({ kind: 'volume', percent: next.percent, muted: next.muted });
        }
      } finally {
        volumeBusy = false;
      }
    };

    // Media drives the idle contextual state, so it needs a slow poll even while the
    // panel is closed. The panel's own refresh takes over as soon as it opens.
    const watchMedia = async () => {
      if (mediaBusy || island.expanded) return;
      mediaBusy = true;
      try {
        island.media = await getNowPlaying();
      } finally {
        mediaBusy = false;
      }
    };

    void listen<string>('key-hud', (event) => {
      if (island.mode !== 'panel') island.flash({ kind: 'key', key: event.payload });
    }).then((off) => {
      unlistenKey = off;
    });

    void listen<NotificationEvent>('notification-hud', (event) => {
      island.recordNotification(event.payload);
      if (island.notifications && island.mode !== 'panel') {
        island.announce({ kind: 'notification', ...event.payload });
      }
    }).then((off) => {
      unlistenNotification = off;
    });

    // The deadline is watched in Rust, so this is the only place that learns a timer
    // ended — including when the panel was never opened and the island was hidden.
    void listen<TimerInfo>('timer-done', (event) => {
      island.timerAlarming = true;
      island.setTimer(event.payload);
      island.announce({ kind: 'timer', ...event.payload });
      chime();
    }).then((off) => {
      unlistenTimer = off;
    });

    void listen<number>('timer-alarm', () => {
      chime();
      if (island.timer?.status === 'done') {
        island.timerAlarming = true;
        if (island.mode !== 'panel') {
          island.announce({ kind: 'timer', ...island.timer });
        }
      }
    }).then((off) => {
      unlistenAlarm = off;
    });

    // The schedule is watched in Rust, so this is the only place that learns a reminder
    // came due — including while the island is hidden or a game holds the screen.
    void listen<ReminderEvent>('reminder-fired', (event) => {
      island.announce({ kind: 'reminder', ...event.payload });
      chime();
    }).then((off) => {
      unlistenReminder = off;
    });

    // A countdown can be mid-flight when the app starts (or restarts) with one still
    // running, so the pill picks up whatever deadline the backend holds.
    void getTimer().then((info) => island.setTimer(info));

    // The Pengingat page reads from the same file the watcher woke up with.
    void getReminders().then((list) => {
      island.reminders = list;
    });

    // The idle "Notifikasi" style should already have content after a restart, so it
    // backfills from the same WPN history the live listener reads.
    if (island.notifications) {
      void getRecentNotifications().then((list) => island.seedNotifications(list));
    }

    // The settings window owns settings.json; every change comes back here so the
    // pill re-themes, re-anchors and re-times itself without a restart.
    void listen<Settings>('settings-changed', (event) => {
      const moved =
        event.payload.anchor !== island.position ||
        event.payload.top_margin !== island.topMargin;
      island.applySettings(event.payload);
      if (moved) void setIslandPosition(event.payload.anchor, event.payload.top_margin);
      if (event.payload.idle === 'music') void watchMedia();
      else if (event.payload.idle === 'mon' || event.payload.idle === 'battery') void refresh();
    }).then((off) => {
      unlistenSettings = off;
    });

    void refresh();
    void watchVolume();
    void watchMedia();
    // A setTimeout chain instead of setInterval so a changed poll_ms takes effect
    // on the very next cycle.
    let poll = 0;
    const loop = () => {
      poll = setTimeout(() => {
        void refresh();
        loop();
      }, island.pollMs);
    };
    loop();
    const volumePoll = setInterval(() => void watchVolume(), 500);
    const mediaPoll = setInterval(() => void watchMedia(), 3000);

    return () => {
      clearTimeout(poll);
      clearInterval(volumePoll);
      clearInterval(mediaPoll);
      unlistenKey?.();
      unlistenNotification?.();
      unlistenSettings?.();
      unlistenTimer?.();
      unlistenAlarm?.();
      unlistenReminder?.();
      void audio?.close();
    };
  });
</script>

<main
  class="flex h-screen w-screen items-start p-8 {island.position === 'left'
    ? 'justify-start'
    : island.position === 'right'
      ? 'justify-end'
      : 'justify-center'}"
>
  <Island />
</main>
