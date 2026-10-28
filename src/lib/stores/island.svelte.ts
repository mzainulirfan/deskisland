import type {
  AppShortcut,
  DownloadEvent,
  Favorite,
  IdleStyle,
  IdleVariant,
  NotificationEvent,
  NowPlaying,
  ProcessInfo,
  Reminder,
  ReminderEvent,
  SystemStats,
  TimerInfo,
  Volume,
} from '$lib/types/system';
import { getSettings, type Settings } from '$lib/services/desktop';

export type IslandMode = 'idle' | 'hud' | 'notice' | 'panel';
export type IslandPosition = 'left' | 'center' | 'right';
export type IslandTheme = 'dark' | 'light';
export type PanelTab =
  | 'system'
  | 'media'
  | 'timer'
  | 'reminders'
  | 'processes'
  | 'downloads'
  | 'apps'
  | 'quick';

export type PomodoroPhase = 'work' | 'shortBreak' | 'longBreak';
export type TimerMode = 'pomodoro' | 'timer';

export interface Pomodoro {
  mode: TimerMode;
  phase: PomodoroPhase;
  /** 1..4, the focus round inside the current cycle. */
  round: number;
  workMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  /** Local date the two counters below belong to; a new day starts them over. */
  day: string;
  sessions: number;
  focusMinutes: number;
}

const POMO_KEY = 'desk_island_pomo_state';
const POMO_PHASES: PomodoroPhase[] = ['work', 'shortBreak', 'longBreak'];

function todayKey() {
  const at = new Date();
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`;
}

/** Minutes survive a corrupt or hand-edited store as a sane whole number. */
function storedMinutes(value: unknown, fallback: number) {
  const num = Number(value);
  return Number.isFinite(num) && num > 0 ? clamp(Math.round(num), 1, 24 * 60) : fallback;
}

function readPomo(): Pomodoro {
  let saved: Partial<Pomodoro> = {};
  try {
    saved = JSON.parse(localStorage.getItem(POMO_KEY) ?? '{}') as Partial<Pomodoro>;
  } catch {
    saved = {};
  }
  const today = todayKey();
  const sameDay = saved.day === today;
  return {
    mode: saved.mode === 'timer' ? 'timer' : 'pomodoro',
    phase: POMO_PHASES.includes(saved.phase as PomodoroPhase)
      ? (saved.phase as PomodoroPhase)
      : 'work',
    round: clamp(Math.round(saved.round ?? 1) || 1, 1, 4),
    workMinutes: storedMinutes(saved.workMinutes, 25),
    shortBreakMinutes: storedMinutes(saved.shortBreakMinutes, 5),
    longBreakMinutes: storedMinutes(saved.longBreakMinutes, 15),
    day: today,
    sessions: sameDay ? Math.max(0, Math.round(saved.sessions ?? 0) || 0) : 0,
    focusMinutes: sameDay ? Math.max(0, Math.round(saved.focusMinutes ?? 0) || 0) : 0,
  };
}

function writePomo(pomo: Pomodoro) {
  try {
    localStorage.setItem(POMO_KEY, JSON.stringify(pomo));
  } catch {
    // Private mode or a full quota: the cycle keeps working for this session.
  }
}

export const IDLE_ORDER: IdleVariant[] = [
  'eyes',
  'min',
  'music',
  'mon',
  'timer',
  'battery',
  'notification',
];
const NOTIFICATION_LOG = 8;

export type IslandHud =
  | { kind: 'volume'; percent: number; muted: boolean }
  | { kind: 'key'; key: string };
export type IslandNotice =
  | ({ kind: 'download' } & DownloadEvent)
  | ({ kind: 'notification' } & NotificationEvent)
  | ({ kind: 'timer' } & TimerInfo)
  | ({ kind: 'reminder' } & ReminderEvent);
export type IslandOverlay = IslandHud | IslandNotice;

/// Where each pill points: the merged media page carries playback plus system volume, so
/// both a volume flash and the music style land there. Downloads have their own page;
/// key flashes and Windows notices open the Windows-side toggles under "Cepat".
const OVERLAY_TAB: Record<IslandOverlay['kind'], PanelTab> = {
  download: 'downloads',
  volume: 'media',
  key: 'quick',
  notification: 'quick',
  timer: 'timer',
  reminder: 'reminders',
};

/// Idle styles that describe a subject; `eyes` and `min` are deliberately absent because
/// they show the clock and system chips, which is what the panel already defaults to.
const IDLE_TAB: Partial<Record<IdleVariant, PanelTab>> = {
  music: 'media',
  timer: 'timer',
  notification: 'quick',
  mon: 'system',
  battery: 'system',
};

function applyTheme(theme: IslandTheme) {
  document.documentElement.dataset.theme = theme;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

class IslandStore {
  mode = $state<IslandMode>('idle');
  panelTab = $state<PanelTab>('system');
  navLayout = $state<'rail' | 'tabs'>('rail');
  pinned = $state(false);
  fullscreen = $state(false);
  time = $state('--:--');
  stats = $state<SystemStats | null>(null);
  volume = $state<Volume | null>(null);
  volumeEditing = $state(false);
  media = $state<NowPlaying | null>(null);
  processes = $state<ProcessInfo[]>([]);
  downloads = $state(0);
  downloadList = $state<DownloadEvent[]>([]);
  favorites = $state<Favorite[]>([]);
  appShortcuts = $state<AppShortcut[]>([]);
  appIcons = $state<Record<string, string>>({});
  /** Daily alarms owned by Rust: this is the same list, read back after every save. */
  reminders = $state<Reminder[]>([]);
  notice = $state<IslandNotice | null>(null);
  noticeUntil = $state(0);
  hud = $state<IslandHud | null>(null);
  hudUntil = $state(0);
  position = $state<IslandPosition>('center');
  /** Logical gap between the screen's top edge and the pill. */
  topMargin = $state(16);
  theme = $state<IslandTheme>('dark');
  idleStyle = $state<IdleStyle>('eyes');
  idleVariant = $state<IdleVariant>('eyes');
  greetingMode = $state<'auto' | 'custom'>('auto');
  greetingText = $state('');
  greetingText2 = $state('');
  greetingShowLine2 = $state(true);
  /** Custom greetings for the "island came back" appearances, one per line. */
  greetingPool = $state('');
  /** Mascot size as a multiplier on the 34px base (settings store the percentage). */
  mascotScale = $state(1.2);
  /** Idle-greeting overlay duration, independent from the pill morph timing. */
  welcomeMs = $state(6000);
  /** Corner rounding in px: collapsed pill vs opened panel. */
  idleRadius = $state(25);
  panelRadius = $state(26);
  /** Width/height morph between idle variants. */
  morphMs = $state(420);
  /** The countdown owned by Rust; null only before the first read. */
  timer = $state<TimerInfo | null>(null);
  /** Minutes the pill and the timer page arm themselves with when nothing runs. */
  timerMinutes = $state(25);
  /**
   * The Pomodoro cycle lives here rather than inside the timer panel, so the pill, the
   * panel and anything added later all read the same round instead of one component's
   * mount state. localStorage keeps it across restarts.
   */
  pomo = $state<Pomodoro>(readPomo());
  /** True while the Rust alarm loop is chiming — the notice stays visible until dismissed. */
  timerAlarming = $state(false);
  /** Wall clock in ms, refreshed only while a countdown is on screen. */
  now = $state(Date.now());
  cursor = $state({ x: 0, y: 0 });
  notifications = $state(true);
  notificationLog = $state<NotificationEvent[]>([]);
  hudMs = $state(1200);
  noticeMs = $state(5800);
  pollMs = $state(1000);

  get expanded() {
    return this.mode === 'panel';
  }

  get activeIdle(): IdleVariant {
    // A live countdown outranks the rotation: remaining time is the one thing the pill
    // should never hide behind a clock.
    if (this.timerLive) return 'timer';
    return this.idleStyle === 'auto' ? this.idleVariant : this.idleStyle;
  }

  /// Running or paused. A finished timer is announced as a notice instead, so the pill
  /// goes back to its usual content once that card has been seen.
  get timerLive(): boolean {
    const status = this.timer?.status;
    return status === 'running' || status === 'paused';
  }

  get timerLeft(): number {
    const timer = this.timer;
    if (!timer) return 0;
    if (timer.status === 'running') return Math.max(0, timer.endsAt - this.now);
    return timer.status === 'paused' ? Math.max(0, timer.remaining) : 0;
  }

  pomoMinutes(phase: PomodoroPhase = this.pomo.phase): number {
    if (phase === 'work') return this.pomo.workMinutes;
    if (phase === 'shortBreak') return this.pomo.shortBreakMinutes;
    return this.pomo.longBreakMinutes;
  }

  /// Every Pomodoro edit writes through: the store is the only place that knows the cycle,
  /// and localStorage is its mirror.
  updatePomo(patch: Partial<Pomodoro>) {
    this.pomo = { ...this.pomo, ...patch, day: todayKey() };
    writePomo(this.pomo);
  }

  /// Work -> break (the long one after round 4), break -> the next round's work.
  /// `completed` is false when the user skips a phase, so today's counters only ever
  /// credit a focus session that actually ran out.
  nextPomoPhase(completed: boolean): PomodoroPhase {
    const pomo = this.pomo;
    const patch: Partial<Pomodoro> = {};
    if (completed && pomo.phase === 'work') {
      patch.sessions = pomo.sessions + 1;
      patch.focusMinutes = pomo.focusMinutes + pomo.workMinutes;
    }
    if (pomo.phase === 'work') {
      patch.phase = pomo.round >= 4 ? 'longBreak' : 'shortBreak';
    } else if (pomo.phase === 'shortBreak') {
      patch.round = Math.min(4, pomo.round + 1);
      patch.phase = 'work';
    } else {
      patch.round = 1;
      patch.phase = 'work';
    }
    this.updatePomo(patch);
    return this.pomo.phase;
  }

  setTimer(info: TimerInfo | null) {
    this.timer = info;
    // A start, resume or cancel leaves the alarm phase — only `done` keeps it.
    if (info && info.status !== 'done') this.timerAlarming = false;
    // A start or resume moves the deadline, so the derived countdown must not compare
    // against a clock that stopped ticking while nothing ran.
    this.now = Date.now();
  }

  /// Stop the alarm loop: the notice disappears and the Rust thread sees `Idle`,
  /// silencing the system beep.
  dismissTimerAlarm() {
    this.timerAlarming = false;
    if (this.notice?.kind === 'timer') {
      this.noticeUntil = 0;
      this.notice = null;
      if (this.mode === 'notice') this.mode = 'idle';
    }
  }

  cycleIdle() {
    if (this.idleStyle !== 'auto') return;
    const index = IDLE_ORDER.indexOf(this.idleVariant);
    this.idleVariant = IDLE_ORDER[(index + 1) % IDLE_ORDER.length];
  }

  /// What the pill should show right now: a transient volume/key flash outranks the
  /// notice (download or Windows notification), which keeps its own deadline and
  /// reappears when the flash ends.
  get overlay(): IslandOverlay | null {
    if (this.hud && Date.now() < this.hudUntil) return this.hud;
    if (
      this.notice &&
      (Date.now() < this.noticeUntil || (this.notice.kind === 'timer' && this.timerAlarming))
    ) {
      return this.notice;
    }
    return null;
  }

  /// HUD (volume/key) stays pill-sized like idle; notices keep the wide card.
  get overlayMode(): IslandMode {
    const overlay = this.overlay;
    if (!overlay) return 'idle';
    return overlay.kind === 'volume' || overlay.kind === 'key' ? 'hud' : 'notice';
  }

  /// The page a pill belongs to, so hovering one opens its subject instead of always
  /// landing on System. An overlay outranks the idle style it replaced, and a plain
  /// clock/eyes pill — which has no subject — reopens wherever the user left the panel.
  get contentTab(): PanelTab {
    const overlay = this.overlay;
    if (overlay) return OVERLAY_TAB[overlay.kind];
    return IDLE_TAB[this.activeIdle] ?? this.panelTab;
  }

  /// A new notice wins over any flash that is still on screen.
  announce(notice: IslandNotice) {
    this.notice = notice;
    this.noticeUntil = Date.now() + this.noticeMs;
    this.hud = null;
    if (this.mode === 'idle' || this.mode === 'hud') this.mode = 'notice';
  }

  /// Newest first; the idle "Notifikasi" style rotates through this list.
  recordNotification(notice: NotificationEvent) {
    if (!this.notifications) return;
    this.notificationLog = [notice, ...this.notificationLog].slice(0, NOTIFICATION_LOG);
  }

  /// Backfills from the WPN database so the style is useful right after launch.
  seedNotifications(list: NotificationEvent[]) {
    if (!this.notifications || list.length === 0) return;
    const key = (notice: NotificationEvent) => `${notice.app}\u0000${notice.title}\u0000${notice.body}`;
    const seen = new Set(this.notificationLog.map(key));
    const fresh = list.filter((notice) => !seen.has(key(notice)));
    if (fresh.length === 0) return;
    this.notificationLog = [...this.notificationLog, ...fresh].slice(0, NOTIFICATION_LOG);
  }

  flash(hud: IslandHud) {
    this.hud = hud;
    this.hudUntil = Date.now() + this.hudMs;
    if (this.mode === 'idle' || this.mode === 'notice') this.mode = 'hud';
  }

  applySettings(settings: Settings) {
    this.theme = settings.theme;
    this.position = settings.anchor;
    this.topMargin = clamp(settings.top_margin ?? 16, 0, 200);
    this.navLayout = settings.nav === 'tabs' ? 'tabs' : 'rail';
    this.idleStyle =
      settings.idle === 'auto' || IDLE_ORDER.includes(settings.idle as IdleVariant)
        ? settings.idle
        : 'eyes';
    if (this.idleStyle !== 'auto') this.idleVariant = this.idleStyle;
    this.greetingMode = settings.greeting_mode === 'custom' ? 'custom' : 'auto';
    this.greetingText = settings.greeting_text ?? '';
    this.greetingText2 = settings.greeting_text2 ?? '';
    this.greetingShowLine2 = settings.greeting_show2 ?? true;
    this.greetingPool = settings.greeting_pool ?? '';
    this.mascotScale = Math.min(1.4, Math.max(0.7, (settings.mascot_scale ?? 120) / 100));
    this.welcomeMs = clamp(settings.welcome_ms ?? 6000, 3500, 20000);
    this.idleRadius = clamp(settings.radius_idle ?? 25, 8, 25);
    this.panelRadius = clamp(settings.radius_panel ?? 26, 8, 40);
    this.morphMs = clamp(settings.morph_ms ?? 420, 150, 900);
    this.notifications = settings.notifications;
    if (!settings.notifications) this.notificationLog = [];
    this.favorites = settings.favorites ?? [];
    this.hudMs = settings.hud_ms;
    this.noticeMs = settings.notice_ms;
    this.pollMs = settings.poll_ms;
    applyTheme(settings.theme);
  }

  async load(): Promise<void> {
    const settings = await getSettings();
    if (settings) this.applySettings(settings);
  }
}

export const island = new IslandStore();
// Applied before the first render so a stored light theme does not flash dark first.
applyTheme(island.theme);
