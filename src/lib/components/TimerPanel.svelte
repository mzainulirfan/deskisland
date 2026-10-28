<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import {
    addTimerTime,
    cancelTimer,
    pauseTimer,
    resumeTimer,
    startTimer,
  } from '$lib/services/desktop';
  import type { TimerInfo } from '$lib/types/system';

  type ActiveMode = 'pomodoro' | 'timer';
  type PomodoroPhase = 'work' | 'shortBreak' | 'longBreak';

  interface TimerPreset {
    minutes: number;
    tag: string;
    label: string;
  }

  const TIMER_PRESETS: TimerPreset[] = [
    { minutes: 1, tag: '⚡ 1m', label: 'Kilat' },
    { minutes: 5, tag: '☕ 5m', label: 'Istirahat' },
    { minutes: 10, tag: '🧘 10m', label: 'Rehat' },
    { minutes: 15, tag: '🍃 15m', label: 'Jeda' },
    { minutes: 25, tag: '🍅 25m', label: 'Pomodoro' },
    { minutes: 30, tag: '🍱 30m', label: 'Makan' },
    { minutes: 45, tag: '🎯 45m', label: 'Fokus' },
    { minutes: 60, tag: '⏱️ 60m', label: '1 Jam' },
  ];

  const POMO_CONFIGS: Record<
    PomodoroPhase,
    { label: string; icon: string; defaultMinutes: number; color: string; desc: string }
  > = {
    work: {
      label: 'Fokus',
      icon: '🍅',
      defaultMinutes: 25,
      color: 'var(--di-red)',
      desc: 'Kerja konsentrasi',
    },
    shortBreak: {
      label: 'Rehat',
      icon: '☕',
      defaultMinutes: 5,
      color: 'var(--di-green)',
      desc: 'Jeda santai',
    },
    longBreak: {
      label: 'Panjang',
      icon: '🌴',
      defaultMinutes: 15,
      color: 'var(--di-blue)',
      desc: 'Setelah 4 sesi',
    },
  };

  const MAX_MINUTES = 24 * 60;
  // Circumference of r=40 in 100 viewBox
  const SWEEP = 2 * Math.PI * 40;

  const formatEnd = new Intl.DateTimeFormat('id-ID', {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });

  const timer = $derived(island.timer);
  const status = $derived(timer?.status ?? 'idle');
  const live = $derived(status === 'running' || status === 'paused');
  const left = $derived(island.timerLeft);
  const alarming = $derived(island.timerAlarming);

  // Active view: Pomodoro as default centerpiece
  let activeMode = $state<ActiveMode>('pomodoro');

  // Pomodoro State
  let pomoPhase = $state<PomodoroPhase>('work');
  let pomoRound = $state(1); // 1 to 4
  let pomoWorkMinutes = $state(25);
  let pomoShortBreakMinutes = $state(5);
  let pomoLongBreakMinutes = $state(15);
  let pomoTodayCount = $state(0);
  let pomoTodayMinutes = $state(0);

  // Standard Timer State
  let timerMinutes = $state(clampMinutes(island.timerMinutes));
  let timerLabel = $state('');

  let busy = $state(false);

  function getTodayKey() {
    return new Date().toISOString().slice(0, 10);
  }

  function loadStorage() {
    try {
      const raw = localStorage.getItem('desk_island_pomo_state');
      if (!raw) return;
      const data = JSON.parse(raw);
      if (data.mode) activeMode = data.mode;
      if (data.workMinutes) pomoWorkMinutes = clampMinutes(data.workMinutes);
      if (data.shortBreakMinutes) pomoShortBreakMinutes = clampMinutes(data.shortBreakMinutes);
      if (data.longBreakMinutes) pomoLongBreakMinutes = clampMinutes(data.longBreakMinutes);
      if (data.round) pomoRound = Math.max(1, Math.min(4, data.round));
      if (data.phase) pomoPhase = data.phase;
      if (data.date === getTodayKey()) {
        pomoTodayCount = data.count || 0;
        pomoTodayMinutes = data.minutes || 0;
      }
    } catch {}
  }

  function saveStorage() {
    try {
      localStorage.setItem(
        'desk_island_pomo_state',
        JSON.stringify({
          mode: activeMode,
          workMinutes: pomoWorkMinutes,
          shortBreakMinutes: pomoShortBreakMinutes,
          longBreakMinutes: pomoLongBreakMinutes,
          round: pomoRound,
          phase: pomoPhase,
          date: getTodayKey(),
          count: pomoTodayCount,
          minutes: pomoTodayMinutes,
        }),
      );
    } catch {}
  }

  onMount(() => {
    loadStorage();
  });

  function clampMinutes(value: number) {
    return Math.max(1, Math.min(MAX_MINUTES, Math.round(value || 1)));
  }

  function getPomoMinutes(phase: PomodoroPhase): number {
    if (phase === 'work') return pomoWorkMinutes;
    if (phase === 'shortBreak') return pomoShortBreakMinutes;
    return pomoLongBreakMinutes;
  }

  function setPomoMinutes(phase: PomodoroPhase, val: number) {
    const clamped = clampMinutes(val);
    if (phase === 'work') pomoWorkMinutes = clamped;
    else if (phase === 'shortBreak') pomoShortBreakMinutes = clamped;
    else pomoLongBreakMinutes = clamped;
    saveStorage();
  }

  function setTimerMinutes(val: number, autoLabel?: string) {
    timerMinutes = clampMinutes(val);
    island.timerMinutes = timerMinutes;
    if (autoLabel && (!timerLabel || TIMER_PRESETS.some((p) => p.label === timerLabel))) {
      timerLabel = autoLabel;
    }
  }

  function getActivePomoLabel(phase: PomodoroPhase, round: number): string {
    if (phase === 'work') return `Fokus (${round}/4)`;
    if (phase === 'shortBreak') return `Istirahat (${round}/4)`;
    return 'Istirahat Panjang';
  }

  const activeDuration = $derived(
    activeMode === 'pomodoro' ? getPomoMinutes(pomoPhase) : timerMinutes,
  );

  const clock = $derived.by(() => {
    const total = Math.ceil((live ? left : activeDuration * 60_000) / 1000);
    const hours = Math.floor(total / 3600);
    const rest = total % 3600;
    const mm = String(hours ? Math.floor(rest / 60) : Math.floor(total / 60)).padStart(2, '0');
    const ss = String(rest % 60).padStart(2, '0');
    return hours ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
  });

  const clockClass = $derived(
    clock.length > 5 ? 'text-[17px] font-bold' : 'text-[21px] font-black',
  );

  const fraction = $derived(
    live
      ? Math.max(0, Math.min(1, left / Math.max(1, timer?.total ?? 1)))
      : status === 'done'
        ? 0
        : 1,
  );

  const percentLeft = $derived(Math.round(fraction * 100));

  // Dynamic Theme Colors based on active phase
  const themeColor = $derived.by(() => {
    if (status === 'done') {
      return alarming ? 'var(--di-amber)' : 'var(--di-green)';
    }
    if (status === 'paused') {
      return 'var(--faint)';
    }
    if (activeMode === 'pomodoro') {
      return POMO_CONFIGS[pomoPhase].color;
    }
    return 'var(--di-amber)';
  });

  const captionText = $derived.by(() => {
    if (status === 'done') return alarming ? 'Waktu habis!' : 'Selesai';
    if (status === 'paused') return 'Dijeda';
    if (live) return timer?.label || 'Berjalan';
    if (activeMode === 'pomodoro') {
      if (pomoPhase === 'work') return `Sesi ${pomoRound} / 4`;
      if (pomoPhase === 'shortBreak') return 'Rehat Singkat';
      return 'Rehat Penuh';
    }
    return `${timerMinutes} mnt siap`;
  });

  const endAt = $derived(status === 'running' ? (timer?.endsAt ?? 0) : island.now + left);
  const endTime = $derived(
    live && endAt > 0 ? `Selesai ± ${formatEnd.format(new Date(endAt))}` : '',
  );

  async function apply(action: () => Promise<TimerInfo | null>) {
    if (busy) return;
    busy = true;
    try {
      const info = await action();
      if (info) island.setTimer(info);
    } finally {
      busy = false;
    }
  }

  function startCountdown(minutes: number, labelText: string) {
    const mins = clampMinutes(minutes);
    void apply(() => startTimer(mins, labelText.trim()));
  }

  function startCurrentSession() {
    if (activeMode === 'pomodoro') {
      const mins = getPomoMinutes(pomoPhase);
      const label = getActivePomoLabel(pomoPhase, pomoRound);
      startCountdown(mins, label);
    } else {
      startCountdown(timerMinutes, timerLabel);
    }
  }

  // Handle transition when Pomodoro phase completes
  function advancePomodoroSession() {
    island.dismissTimerAlarm();
    if (pomoPhase === 'work') {
      pomoTodayCount += 1;
      pomoTodayMinutes += pomoWorkMinutes;
      saveStorage();
      if (pomoRound >= 4) {
        pomoPhase = 'longBreak';
      } else {
        pomoPhase = 'shortBreak';
      }
    } else if (pomoPhase === 'shortBreak') {
      pomoRound = Math.min(4, pomoRound + 1);
      pomoPhase = 'work';
    } else {
      // Completed long break -> restart cycle at round 1
      pomoRound = 1;
      pomoPhase = 'work';
    }
    saveStorage();
    startCurrentSession();
  }

  function skipPhase() {
    island.dismissTimerAlarm();
    void apply(cancelTimer);
    if (pomoPhase === 'work') {
      pomoPhase = pomoRound >= 4 ? 'longBreak' : 'shortBreak';
    } else {
      if (pomoPhase === 'longBreak') pomoRound = 1;
      else pomoRound = Math.min(4, pomoRound + 1);
      pomoPhase = 'work';
    }
    saveStorage();
  }

  function selectPhase(phase: PomodoroPhase) {
    if (live) return;
    pomoPhase = phase;
    saveStorage();
  }

  function resetCycle() {
    if (live) void apply(cancelTimer);
    island.dismissTimerAlarm();
    pomoRound = 1;
    pomoPhase = 'work';
    saveStorage();
  }

  function toggleMode(mode: ActiveMode) {
    if (activeMode === mode) return;
    activeMode = mode;
    saveStorage();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    startCurrentSession();
  }
</script>

<div
  class="timer-shell flex h-full min-h-0 items-stretch gap-3 rounded-[15px] p-2 ring-1 ring-[color:var(--line)]"
  style:--tint={themeColor}
>
  <!-- ================= LEFT: HERO FOCUS DIAL STAGE ================= -->
  <div
    class="hero-card relative flex w-[196px] shrink-0 flex-col items-center justify-between overflow-hidden rounded-[13px] p-2.5 text-center ring-1 ring-[color:var(--line)]"
    class:hero-card-alarming={alarming}
  >
    <!-- Ambient Backlight -->
    <div class="ambient-glow" aria-hidden="true"></div>

    <!-- Circular Dial -->
    <div class="relative flex size-[84px] shrink-0 items-center justify-center">
      <svg
        class="absolute inset-0 size-[84px] -rotate-90 transition-transform duration-300"
        viewBox="0 0 100 100"
        fill="none"
        aria-hidden="true"
      >
        <!-- Background Ring Track -->
        <circle cx="50" cy="50" r="40" stroke="var(--track)" stroke-width="5" />
        <!-- Animated Progress Stroke -->
        <circle
          cx="50"
          cy="50"
          r="40"
          stroke={themeColor}
          stroke-width="5"
          stroke-linecap="round"
          stroke-dasharray={SWEEP}
          stroke-dashoffset={SWEEP * (1 - fraction)}
          class="dial-progress"
        />
      </svg>

      <!-- Center Typography -->
      <div class="relative flex flex-col items-center leading-none">
        <span class="tracking-tight tabular-nums text-[var(--ink)] {clockClass}">{clock}</span>
        <span
          class="badge-pill mt-0.5 max-w-[86px] truncate rounded-full px-2 py-0.5 text-[8px] font-bold uppercase tracking-wider"
          style:background="color-mix(in srgb, var(--tint) 15%, transparent)"
          style:color={themeColor}
        >
          {captionText}
        </span>
      </div>
    </div>

    <!-- Context Hint / Round Indicators -->
    <div class="flex h-4 items-center justify-center">
      {#if activeMode === 'pomodoro'}
        <!-- 4-Session Interactive Dots -->
        <div class="flex items-center gap-1.5" title="Siklus Pomodoro: Sesi {pomoRound} dari 4">
          {#each [1, 2, 3, 4] as r}
            {@const isDone = r < pomoRound}
            {@const isCurrent = r === pomoRound}
            <span
              class="cycle-dot"
              class:cycle-dot-done={isDone}
              class:cycle-dot-current={isCurrent && live}
              class:cycle-dot-ready={isCurrent && !live}
              style:--dot-color={themeColor}
            ></span>
          {/each}
          <span class="ml-1 text-[8.5px] font-bold tabular-nums text-[var(--faint)]">
            {pomoRound}/4
          </span>
        </div>
      {:else if endTime}
        <span class="flex items-center gap-1 text-[9.5px] font-medium tabular-nums text-[var(--muted)]">
          <Icon name="clock" class="size-2.5 text-[var(--faint)]" />
          <span>{endTime}</span>
        </span>
      {:else}
        <span class="text-[9px] font-medium text-[var(--faint)]">
          {live ? `${percentLeft}% sisa` : 'Siap dimulai'}
        </span>
      {/if}
    </div>

    <!-- Master Action CTA Buttons -->
    <div class="flex w-full items-center justify-center gap-1.5">
      {#if live}
        <button
          type="button"
          class="action-btn action-btn-main flex-1"
          style:background={themeColor}
          aria-label={status === 'running' ? 'Jeda sesi' : 'Lanjutkan sesi'}
          disabled={busy}
          onclick={() => void apply(status === 'running' ? pauseTimer : resumeTimer)}
        >
          <Icon name={status === 'running' ? 'pause' : 'play'} class="size-3 fill-current" />
          <span>{status === 'running' ? 'Jeda' : 'Lanjut'}</span>
        </button>

        <button
          type="button"
          class="action-btn action-btn-sub"
          aria-label="Batalkan"
          disabled={busy}
          onclick={() => void apply(cancelTimer)}
        >
          <Icon name="close" class="size-3" />
          <span>Batal</span>
        </button>
      {:else if status === 'done'}
        {#if activeMode === 'pomodoro'}
          <button
            type="button"
            class="action-btn action-btn-main flex-1"
            style:background={themeColor}
            aria-label="Lanjut sesi berikutnya"
            disabled={busy}
            onclick={advancePomodoroSession}
          >
            <Icon name="play" class="size-3 fill-current" />
            <span>Lanjut {pomoPhase === 'work' ? 'Rehat' : 'Fokus'}</span>
          </button>
        {:else}
          <button
            type="button"
            class="action-btn action-btn-main flex-1"
            aria-label="Ulangi timer"
            disabled={busy}
            onclick={() => {
              island.dismissTimerAlarm();
              startCountdown(timerMinutes, timerLabel);
            }}
          >
            <Icon name="timer" class="size-3" />
            <span>Ulangi</span>
          </button>
        {/if}

        <button
          type="button"
          class="action-btn action-btn-sub"
          aria-label="Tutup timer"
          disabled={busy}
          onclick={() => {
            island.dismissTimerAlarm();
            void apply(cancelTimer);
          }}
        >
          <Icon name="close" class="size-3" />
          <span>Tutup</span>
        </button>
      {:else}
        <button
          type="button"
          class="action-btn action-btn-main w-full"
          style:background={themeColor}
          aria-label="Mulai sesi"
          disabled={busy}
          onclick={startCurrentSession}
        >
          <Icon name="play" class="size-3 fill-current" />
          <span>Mulai {activeDuration} mnt</span>
        </button>
      {/if}
    </div>
  </div>

  <!-- ================= RIGHT: INTERACTIVE CONTROL & STATS HUB ================= -->
  <div class="flex min-w-0 flex-1 flex-col justify-between overflow-hidden py-0.5">
    <!-- Top Header: Mode Switcher & Streak Counter -->
    <div class="flex items-center justify-between border-b border-[var(--line)] pb-1.5">
      <div class="segmented-control">
        <button
          type="button"
          class="segmented-tab"
          class:segmented-tab-active={activeMode === 'pomodoro'}
          onclick={() => toggleMode('pomodoro')}
        >
          <span class="text-[10px] leading-none">🍅</span>
          <span>Pomodoro</span>
        </button>
        <button
          type="button"
          class="segmented-tab"
          class:segmented-tab-active={activeMode === 'timer'}
          onclick={() => toggleMode('timer')}
        >
          <Icon name="timer" class="size-2.5" />
          <span>Timer</span>
        </button>
      </div>

      <!-- Streak & Productive Session Badge -->
      {#if activeMode === 'pomodoro'}
        <div class="streak-badge" title="Statistik fokus hari ini">
          <span class="text-[var(--di-red)]">🔥 {pomoTodayCount} sesi</span>
          <span class="opacity-40">·</span>
          <span>{pomoTodayMinutes}m fokus</span>
        </div>
      {:else}
        <span class="text-[9px] font-medium text-[var(--faint)]">
          {live ? `${percentLeft}% tersisa` : 'Hitung Mundur'}
        </span>
      {/if}
    </div>

    <!-- MAIN BODY SECTION -->
    {#if activeMode === 'pomodoro'}
      <!-- ************* POMODORO VIEW ************* -->
      {#if live}
        <!-- POMODORO LIVE COUNTDOWN -->
        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-1.5">
              <span
                class="flex size-6 shrink-0 items-center justify-center rounded-[7px]"
                style:background="color-mix(in srgb, {themeColor} 20%, transparent)"
                style:color={themeColor}
              >
                <Icon name={pomoPhase === 'work' ? 'focus' : 'sparks'} class="size-3" />
              </span>
              <div class="flex min-w-0 flex-col leading-tight">
                <span class="truncate text-[12px] font-bold text-[var(--ink)]">
                  {getActivePomoLabel(pomoPhase, pomoRound)}
                </span>
                <span class="text-[9px] font-medium text-[var(--faint)]">
                  {status === 'running' ? 'Sedang berjalan' : 'Sesi dijeda'} · {percentLeft}% tersisa
                </span>
              </div>
            </div>

            <button
              type="button"
              class="skip-btn"
              onclick={skipPhase}
              title="Lewati fase saat ini"
            >
              Lewati →
            </button>
          </div>

          <!-- Linear Track -->
          <div class="flex flex-col gap-0.5">
            <div class="relative h-1.5 w-full overflow-hidden rounded-full bg-[var(--track)]">
              <div
                class="h-full rounded-full transition-[width] duration-300 ease-linear"
                style:width="{100 - percentLeft}%"
                style:background={themeColor}
              ></div>
            </div>
            <div class="flex justify-between text-[8.5px] tabular-nums text-[var(--faint)]">
              <span>Berjalan {Math.floor(((timer?.total ?? 0) - left) / 60_000)} mnt</span>
              <span>Tersisa {Math.ceil(left / 60_000)} mnt</span>
            </div>
          </div>

          <!-- Live Boosters -->
          <div class="flex items-center gap-1">
            <span class="mr-1 text-[8.5px] font-bold uppercase tracking-wider text-[var(--faint)]">
              Atur:
            </span>
            {#each [-5, -1, 1, 5, 10] as delta}
              <button
                type="button"
                class="booster-chip"
                class:booster-sub={delta < 0}
                disabled={busy}
                onclick={() => void apply(() => addTimerTime(delta))}
              >
                {delta > 0 ? `+${delta}m` : `${delta}m`}
              </button>
            {/each}
          </div>
        </div>

        <div class="flex items-center gap-1.5 rounded-[8px] bg-[var(--chip)] px-2 py-1 text-[9px] text-[var(--faint)] ring-1 ring-[color:var(--line)]">
          <Icon name="sparks" class="size-2.5 shrink-0 text-[var(--di-amber)]" />
          <span class="truncate">Desk Island akan membunyikan alarm saat sesi tuntas.</span>
        </div>
      {:else if status === 'done'}
        <!-- POMODORO COMPLETED BANNER -->
        <div class="flex flex-col gap-2">
          <div
            class="flex items-center gap-2 rounded-[10px] p-2.5 ring-1"
            style:background="color-mix(in srgb, {themeColor} 14%, transparent)"
            style:border-color="color-mix(in srgb, {themeColor} 30%, transparent)"
          >
            <span
              class="flex size-7 shrink-0 items-center justify-center rounded-[8px] text-[#0b0d12]"
              style:background={themeColor}
            >
              <Icon name="check" class="size-3.5" />
            </span>
            <div class="flex min-w-0 flex-1 flex-col leading-tight">
              <span class="text-[12px] font-bold" style:color={themeColor}>
                {alarming ? 'Waktu Telah Habis!' : 'Sesi Tuntas'}
              </span>
              <span class="truncate text-[9.5px] text-[var(--muted)]">
                {#if pomoPhase === 'work'}
                  {pomoRound >= 4
                    ? 'Hebat! 4 Sesi Fokus tuntas. Saatnya rehat panjang!'
                    : `Sesi Fokus #${pomoRound} selesai. Waktunya rehat sejenak.`}
                {:else}
                  Istirahat selesai. Pikiranmu segar kembali untuk sesi berikutnya!
                {/if}
              </span>
            </div>
          </div>

          <div class="flex items-center justify-between rounded-[8px] bg-[var(--chip)] px-2.5 py-1.5 text-[9.5px] ring-1 ring-[color:var(--line)]">
            <span class="text-[var(--muted)]">
              Langkah berikutnya: <strong class="text-[var(--ink)]">
                {pomoPhase === 'work' ? (pomoRound >= 4 ? '🌴 Istirahat Panjang (15m)' : '☕ Rehat (5m)') : `🍅 Fokus Sesi ${pomoRound}/4`}
              </strong>
            </span>
            <button
              type="button"
              class="font-bold text-[var(--di-blue)] hover:underline"
              onclick={advancePomodoroSession}
            >
              Lanjut Sekarang →
            </button>
          </div>
        </div>
      {:else}
        <!-- POMODORO IDLE SETUP -->
        <div class="flex flex-col gap-2">
          <!-- 3 Interactive Phase Cards -->
          <div class="grid grid-cols-3 gap-1.5" role="group" aria-label="Pilih Fase">
            <!-- 1. Fokus -->
            <button
              type="button"
              class="phase-card"
              class:phase-card-active={pomoPhase === 'work'}
              onclick={() => selectPhase('work')}
            >
              <span class="text-[13px]">🍅</span>
              <div class="flex flex-col leading-none">
                <span class="text-[11px] font-bold">Fokus</span>
                <span class="mt-0.5 text-[8.5px] font-semibold opacity-70 tabular-nums">
                  {pomoWorkMinutes} mnt
                </span>
              </div>
            </button>

            <!-- 2. Rehat Singkat -->
            <button
              type="button"
              class="phase-card phase-card-green"
              class:phase-card-active={pomoPhase === 'shortBreak'}
              onclick={() => selectPhase('shortBreak')}
            >
              <span class="text-[13px]">☕</span>
              <div class="flex flex-col leading-none">
                <span class="text-[11px] font-bold">Rehat</span>
                <span class="mt-0.5 text-[8.5px] font-semibold opacity-70 tabular-nums">
                  {pomoShortBreakMinutes} mnt
                </span>
              </div>
            </button>

            <!-- 3. Rehat Panjang -->
            <button
              type="button"
              class="phase-card phase-card-blue"
              class:phase-card-active={pomoPhase === 'longBreak'}
              onclick={() => selectPhase('longBreak')}
            >
              <span class="text-[13px]">🌴</span>
              <div class="flex flex-col leading-none">
                <span class="text-[11px] font-bold">Panjang</span>
                <span class="mt-0.5 text-[8.5px] font-semibold opacity-70 tabular-nums">
                  {pomoLongBreakMinutes} mnt
                </span>
              </div>
            </button>
          </div>

          <!-- Phase Duration Picker & Stepper -->
          <div class="flex items-center justify-between rounded-[9px] bg-[var(--chip)] px-2.5 py-1.5 ring-1 ring-[color:var(--line)]">
            <span class="text-[9.5px] font-medium text-[var(--muted)]">
              Durasi {POMO_CONFIGS[pomoPhase].label}:
            </span>
            <div class="flex items-center gap-1">
              {#each (pomoPhase === 'work' ? [20, 25, 30, 45, 50] : pomoPhase === 'shortBreak' ? [3, 5, 8, 10] : [10, 15, 20, 30]) as d}
                {@const isCurrent = getPomoMinutes(pomoPhase) === d}
                <button
                  type="button"
                  class="dur-pill"
                  class:dur-pill-active={isCurrent}
                  onclick={() => setPomoMinutes(pomoPhase, d)}
                >
                  {d}m
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <!-- Pomodoro Bottom Bar -->
      <div class="flex items-center justify-between border-t border-[var(--line)] pt-1 text-[9px] text-[var(--faint)]">
        <span class="flex items-center gap-1">
          <Icon name="focus" class="size-2.5 text-[var(--di-red)]" />
          Siklus: 4 sesi fokus bergantian dengan istirahat
        </span>
        <button
          type="button"
          class="font-medium text-[var(--muted)] hover:text-[var(--ink)]"
          onclick={resetCycle}
          title="Reset putaran ke Sesi 1"
        >
          Reset Siklus
        </button>
      </div>
    {:else}
      <!-- ************* STANDARD TIMER VIEW ************* -->
      {#if live}
        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <div class="flex min-w-0 items-center gap-1.5">
              <span class="flex size-5 shrink-0 items-center justify-center rounded-[6px] bg-[color:color-mix(in_srgb,var(--di-amber)_18%,transparent)] text-[var(--di-amber)]">
                <Icon name="timer" class="size-3" />
              </span>
              <div class="flex min-w-0 flex-col leading-tight">
                <span class="truncate text-[12px] font-semibold text-[var(--ink)]">
                  {timer?.label || 'Timer'}
                </span>
                <span class="text-[9px] font-medium text-[var(--faint)]">
                  Total {Math.round((timer?.total ?? 0) / 60_000)} mnt · {status === 'running' ? 'Aktif' : 'Dijeda'}
                </span>
              </div>
            </div>

            <span class="rounded-full bg-[var(--chip)] px-2 py-0.5 text-[9.5px] font-bold tabular-nums text-[var(--di-amber)] ring-1 ring-[color:var(--line)]">
              {percentLeft}%
            </span>
          </div>

          <div class="flex flex-col gap-0.5">
            <div class="relative h-1.5 w-full overflow-hidden rounded-full bg-[var(--track)]">
              <div
                class="h-full rounded-full transition-[width] duration-300 ease-linear"
                style:width="{100 - percentLeft}%"
                style:background="var(--di-amber)"
              ></div>
            </div>
            <div class="flex justify-between text-[8.5px] tabular-nums text-[var(--faint)]">
              <span>Berjalan {Math.floor(((timer?.total ?? 0) - left) / 60_000)} mnt</span>
              <span>Tersisa {Math.ceil(left / 60_000)} mnt</span>
            </div>
          </div>

          <div class="flex items-center gap-1">
            <span class="mr-1 text-[8.5px] font-bold uppercase tracking-wider text-[var(--faint)]">
              Atur:
            </span>
            {#each [-5, -1, 1, 5, 10, 15] as delta}
              <button
                type="button"
                class="booster-chip"
                class:booster-sub={delta < 0}
                disabled={busy}
                onclick={() => void apply(() => addTimerTime(delta))}
              >
                {delta > 0 ? `+${delta}m` : `${delta}m`}
              </button>
            {/each}
          </div>
        </div>
      {:else if status === 'done'}
        <div class="flex flex-col gap-2">
          <div class="flex items-center gap-2 rounded-[9px] bg-[color:color-mix(in_srgb,var(--di-amber)_14%,transparent)] p-2 ring-1 ring-[color:color-mix(in_srgb,var(--di-amber)_28%,transparent)]">
            <span class="flex size-7 shrink-0 items-center justify-center rounded-[7px] bg-[var(--di-amber)] text-[#0b0d12]">
              <Icon name="timer" class="size-3.5" />
            </span>
            <div class="flex min-w-0 flex-1 flex-col leading-tight">
              <span class="text-[12px] font-bold text-[var(--di-amber)]">
                {alarming ? 'Waktu Telah Habis!' : 'Timer Selesai'}
              </span>
              <span class="truncate text-[9.5px] text-[var(--muted)]">
                Hitung mundur {timerMinutes} menit telah tuntas.
              </span>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-1.5">
            {#each TIMER_PRESETS.slice(1, 5) as p}
              <button
                type="button"
                class="quick-preset-card"
                disabled={busy}
                onclick={() => {
                  island.dismissTimerAlarm();
                  setTimerMinutes(p.minutes, p.label);
                  startCountdown(p.minutes, p.label);
                }}
              >
                <span class="font-bold text-[var(--ink)]">{p.tag}</span>
                <span class="truncate text-[8.5px] text-[var(--muted)]">{p.label}</span>
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <!-- TIMER IDLE SETUP -->
        <div class="flex flex-col gap-2">
          <div class="grid grid-cols-[1fr_auto] gap-1.5">
            <!-- Custom Label Input -->
            <div class="relative flex items-center">
              <input
                class="timer-input w-full pr-6"
                type="text"
                maxlength="28"
                placeholder="Nama sesi (opsional)..."
                aria-label="Nama pengingat"
                bind:value={timerLabel}
                onkeydown={onKeydown}
              />
              {#if timerLabel}
                <button
                  type="button"
                  class="absolute right-1.5 flex size-4 items-center justify-center rounded-full text-[var(--faint)] hover:text-[var(--ink)]"
                  onclick={() => (timerLabel = '')}
                  aria-label="Hapus nama"
                >
                  <Icon name="close" class="size-2.5" />
                </button>
              {/if}
            </div>

            <!-- Stepper: [-] [Minutes Input] [+] -->
            <div class="flex items-center gap-0.5 rounded-[8px] bg-[var(--chip)] p-0.5 ring-1 ring-[color:var(--line)]">
              <button
                type="button"
                class="stepper-btn"
                aria-label="Kurangi 1 menit"
                onclick={() => setTimerMinutes(timerMinutes - 1)}
                disabled={timerMinutes <= 1}
              >
                -
              </button>
              <div class="flex items-center px-1">
                <input
                  class="w-[30px] border-0 bg-transparent text-center font-bold tabular-nums text-[11.5px] text-[var(--ink)] focus:outline-none"
                  type="number"
                  min="1"
                  max={MAX_MINUTES}
                  step="1"
                  aria-label="Durasi menit"
                  value={timerMinutes}
                  oninput={(event) => setTimerMinutes(Number((event.currentTarget as HTMLInputElement).value))}
                  onkeydown={onKeydown}
                />
                <span class="text-[8.5px] font-semibold uppercase text-[var(--faint)]">m</span>
              </div>
              <button
                type="button"
                class="stepper-btn"
                aria-label="Tambah 1 menit"
                onclick={() => setTimerMinutes(timerMinutes + 1)}
                disabled={timerMinutes >= MAX_MINUTES}
              >
                +
              </button>
            </div>
          </div>

          <!-- Presets Grid -->
          <div class="grid grid-cols-4 gap-1" role="group" aria-label="Preset timer">
            {#each TIMER_PRESETS as preset (preset.minutes)}
              {@const selected = timerMinutes === preset.minutes}
              <button
                type="button"
                class="preset-card"
                class:preset-card-on={selected}
                aria-pressed={selected}
                onclick={() => setTimerMinutes(preset.minutes, preset.label)}
              >
                <span class="truncate text-[10.5px] font-bold leading-none">{preset.tag}</span>
                <span class="truncate text-[8px] font-medium opacity-75">{preset.label}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Timer Footer -->
      <div class="flex items-center justify-between border-t border-[var(--line)] pt-1 text-[9px] text-[var(--faint)]">
        <span class="flex items-center gap-1">
          <Icon name="sparks" class="size-2.5 text-[var(--di-amber)]" />
          Tekan <kbd class="rounded bg-[var(--chip)] px-1 py-0.5 text-[8px] font-semibold text-[var(--muted)]">Enter</kbd> untuk mulai langsung
        </span>
        <span class="font-medium text-[var(--muted)]">Maks. 24 Jam</span>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Base Shell */
  .timer-shell {
    background:
      linear-gradient(
        180deg,
        color-mix(in srgb, var(--tint) 7%, transparent),
        transparent 55%
      ),
      var(--card);
  }

  /* Left Hero Card */
  .hero-card {
    background:
      radial-gradient(
        120% 90% at 50% 0%,
        color-mix(in srgb, var(--tint) 14%, transparent),
        transparent 65%
      ),
      var(--di-surface);
  }

  .hero-card-alarming {
    animation: alarm-glow-pulse 1.8s infinite ease-in-out;
  }

  @keyframes alarm-glow-pulse {
    0%,
    100% {
      box-shadow: inset 0 0 0 1px var(--di-amber);
    }
    50% {
      box-shadow:
        inset 0 0 0 1px var(--di-amber),
        0 0 20px color-mix(in srgb, var(--di-amber) 40%, transparent);
    }
  }

  .dial-progress {
    transition: stroke-dashoffset 240ms linear;
    filter: drop-shadow(0 0 6px color-mix(in srgb, var(--tint) 45%, transparent));
  }

  /* 4-Session Interactive Dots */
  .cycle-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--track);
    transition: all 220ms var(--ease-out);
  }

  .cycle-dot-done {
    background: var(--dot-color);
    box-shadow: 0 0 6px var(--dot-color);
  }

  .cycle-dot-current {
    background: var(--dot-color);
    box-shadow: 0 0 8px var(--dot-color);
    transform: scale(1.2);
  }

  .cycle-dot-ready {
    border: 1.5px solid var(--dot-color);
    background: transparent;
  }

  /* Top Segmented Mode Switcher */
  .segmented-control {
    display: inline-flex;
    align-items: center;
    background: var(--chip);
    padding: 2px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .segmented-tab {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 8px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    color: var(--muted);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .segmented-tab:hover {
    color: var(--ink);
  }

  .segmented-tab-active {
    background: var(--di-surface-hover);
    color: var(--ink);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  }

  .streak-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 9.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }

  /* Pomodoro Phase Cards */
  .phase-card {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    border-radius: 9px;
    background: var(--chip);
    padding: 0 8px;
    text-align: left;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out),
      box-shadow 120ms var(--ease-out),
      transform 100ms var(--ease-out);
  }

  .phase-card:hover {
    color: var(--ink);
    background: var(--di-surface-hover);
    transform: translateY(-1px);
  }

  .phase-card-active {
    color: var(--di-red);
    background: color-mix(in srgb, var(--di-red) 16%, transparent);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-red) 50%, transparent),
      0 0 12px color-mix(in srgb, var(--di-red) 20%, transparent);
  }

  .phase-card-green.phase-card-active {
    color: var(--di-green);
    background: color-mix(in srgb, var(--di-green) 16%, transparent);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-green) 50%, transparent),
      0 0 12px color-mix(in srgb, var(--di-green) 20%, transparent);
  }

  .phase-card-blue.phase-card-active {
    color: var(--di-blue);
    background: color-mix(in srgb, var(--di-blue) 16%, transparent);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-blue) 50%, transparent),
      0 0 12px color-mix(in srgb, var(--di-blue) 20%, transparent);
  }

  /* Duration Selection Pills */
  .dur-pill {
    padding: 2px 7px;
    border-radius: 6px;
    font-size: 9.5px;
    font-weight: 700;
    color: var(--muted);
    background: transparent;
    transition: all 100ms var(--ease-out);
  }

  .dur-pill:hover {
    color: var(--ink);
    background: var(--di-surface-hover);
  }

  .dur-pill-active {
    color: var(--ink);
    background: var(--active);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  /* Action Buttons */
  .action-btn {
    display: inline-flex;
    height: 28px;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border-radius: 8px;
    padding: 0 10px;
    font-size: 11px;
    font-weight: 600;
    transition:
      transform 120ms var(--ease-out),
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .action-btn:hover {
    transform: translateY(-1px);
  }

  .action-btn:active {
    transform: translateY(0);
  }

  .action-btn-main {
    color: #0b0d12;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--tint) 35%, transparent);
  }

  .action-btn-main:hover {
    filter: brightness(1.1);
  }

  .action-btn-sub {
    color: var(--muted);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .action-btn-sub:hover {
    color: var(--ink);
    background: var(--di-surface-hover);
  }

  .skip-btn {
    border-radius: 6px;
    background: var(--chip);
    padding: 3px 8px;
    font-size: 9px;
    font-weight: 700;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: all 100ms var(--ease-out);
  }

  .skip-btn:hover {
    color: var(--ink);
    background: var(--di-surface-hover);
  }

  /* Booster Chips (+/-) */
  .booster-chip {
    height: 24px;
    padding: 0 8px;
    border-radius: 7px;
    background: var(--chip);
    color: var(--ink);
    font-size: 10.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out),
      transform 100ms var(--ease-out);
  }

  .booster-chip:hover {
    color: var(--tint);
    background: color-mix(in srgb, var(--tint) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tint) 32%, transparent);
    transform: translateY(-1px);
  }

  .booster-sub {
    color: var(--muted);
  }

  .booster-sub:hover {
    color: var(--di-red);
    background: color-mix(in srgb, var(--di-red) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-red) 30%, transparent);
  }

  /* Preset Card */
  .preset-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: 32px;
    border-radius: 8px;
    background: var(--chip);
    padding: 0 4px;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out),
      transform 100ms var(--ease-out);
  }

  .preset-card:hover {
    color: var(--ink);
    background: var(--di-surface-hover);
    transform: translateY(-1px);
  }

  .preset-card-on {
    color: var(--di-amber);
    background: color-mix(in srgb, var(--di-amber) 16%, transparent);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 50%, transparent),
      0 0 10px color-mix(in srgb, var(--di-amber) 18%, transparent);
  }

  .quick-preset-card {
    display: flex;
    flex-direction: column;
    justify-content: center;
    height: 32px;
    border-radius: 8px;
    background: var(--chip);
    padding: 0 8px;
    text-align: left;
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      background 120ms var(--ease-out),
      transform 100ms var(--ease-out);
  }

  .quick-preset-card:hover {
    background: var(--di-surface-hover);
    transform: translateY(-1px);
  }

  /* Stepper Buttons */
  .stepper-btn {
    display: flex;
    width: 22px;
    height: 22px;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    font-weight: bold;
    transition:
      background 100ms var(--ease-out),
      color 100ms var(--ease-out);
  }

  .stepper-btn:hover:not(:disabled) {
    background: var(--di-surface-hover);
    color: var(--ink);
  }

  .stepper-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  /* Input fields */
  .timer-input {
    height: 28px;
    border: 0;
    border-radius: 8px;
    background: var(--chip);
    padding: 0 8px;
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    transition: box-shadow 120ms var(--ease-out);
  }

  .timer-input::placeholder {
    color: var(--faint);
  }

  .timer-input:focus-visible {
    outline: none;
    box-shadow:
      inset 0 0 0 1px var(--tint),
      0 0 8px color-mix(in srgb, var(--tint) 25%, transparent);
  }

  input[type='number']::-webkit-inner-spin-button,
  input[type='number']::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
</style>
