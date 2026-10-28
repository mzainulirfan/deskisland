<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
  import Icon from '$lib/components/Icon.svelte';
  import type { IconName } from '$lib/components/Icon.svelte';
  import { getSettings, setSettings, getAutostart, setAutostart, type Settings } from '$lib/services/desktop';
  import { GREETING_PRESETS } from '$lib/greetings';
  import type { IdleStyle } from '$lib/types/system';

  const DEFAULTS: Settings = {
    theme: 'dark',
    anchor: 'center',
    top_margin: 16,
    idle: 'eyes',
    nav: 'rail',
    greeting_mode: 'auto',
    greeting_text: '',
    greeting_text2: '',
    greeting_show2: true,
    greeting_pool: '',
    mascot_scale: 120,
    radius_idle: 25,
    radius_panel: 26,
    welcome_ms: 6000,
    morph_ms: 420,
    notifications: true,
    hud_ms: 1200,
    notice_ms: 5800,
    poll_ms: 1000,
  };

  type SectionId = 'appearance' | 'greeting' | 'behavior' | 'integration';
  const SECTIONS: { id: SectionId; label: string; hint: string; icon: IconName }[] = [
    { id: 'appearance', label: 'Tampilan', hint: 'Tema, posisi, pill', icon: 'palette' },
    { id: 'greeting', label: 'Sambutan', hint: 'Pesan pada pill', icon: 'edit' },
    { id: 'behavior', label: 'Perilaku', hint: 'Durasi & ritme', icon: 'sliders' },
    { id: 'integration', label: 'Integrasi', hint: 'Hubungkan ke Windows', icon: 'plug' },
  ];

  const THEME_OPTIONS: { value: Settings['theme']; label: string; icon: IconName }[] = [
    { value: 'dark', label: 'Gelap', icon: 'moon' },
    { value: 'light', label: 'Terang', icon: 'sun' },
  ];

  const ANCHOR_OPTIONS: { value: Settings['anchor']; label: string; icon: IconName }[] = [
    { value: 'left', label: 'Kiri', icon: 'align-left' },
    { value: 'center', label: 'Tengah', icon: 'align-center' },
    { value: 'right', label: 'Kanan', icon: 'align-right' },
  ];

  const NAV_OPTIONS: { value: Settings['nav']; label: string; icon: IconName }[] = [
    { value: 'rail', label: 'Rail', icon: 'layout-rail' },
    { value: 'tabs', label: 'Tab', icon: 'layout-tabs' },
  ];

  const GREETING_OPTIONS: {
    value: Settings['greeting_mode'];
    label: string;
    icon: IconName;
    desc: string;
  }[] = [
    { value: 'auto', label: 'Bawaan', icon: 'sparks', desc: 'Acak sesuai jam' },
    { value: 'custom', label: 'Kustom', icon: 'edit', desc: 'Tulis pesanmu' },
  ];

  const IDLE_OPTIONS: { value: IdleStyle; label: string; icon: IconName }[] = [
    { value: 'eyes', label: 'Mata', icon: 'eye' },
    { value: 'min', label: 'Minimal', icon: 'clock' },
    { value: 'music', label: 'Musik', icon: 'media' },
    { value: 'mon', label: 'Monitor', icon: 'monitor' },
    { value: 'timer', label: 'Timer', icon: 'timer' },
    { value: 'battery', label: 'Baterai', icon: 'battery' },
    { value: 'notification', label: 'Notifikasi', icon: 'bell' },
    { value: 'auto', label: 'Otomatis', icon: 'sparks' },
  ];

  type BehaviorKey = 'hud_ms' | 'notice_ms' | 'poll_ms' | 'morph_ms';
  const BEHAVIOR_ROWS: {
    key: BehaviorKey;
    label: string;
    hint: string;
    icon: IconName;
    min: number;
    max: number;
    step: number;
  }[] = [
    {
      key: 'hud_ms',
      label: 'Durasi HUD',
      hint: 'Volume, kecerahan, tombol media',
      icon: 'volume',
      min: 500,
      max: 3000,
      step: 100,
    },
    {
      key: 'notice_ms',
      label: 'Durasi notifikasi',
      hint: 'Lama toast bertahan di pill',
      icon: 'bell',
      min: 2000,
      max: 12000,
      step: 250,
    },
    {
      key: 'poll_ms',
      label: 'Interval polling',
      hint: 'Seberapa sering statistik dibaca',
      icon: 'activity',
      min: 500,
      max: 5000,
      step: 100,
    },
    {
      key: 'morph_ms',
      label: 'Durasi ganti varian',
      hint: 'Pill melebar saat tampilan berganti',
      icon: 'sliders',
      min: 150,
      max: 900,
      step: 10,
    },
  ];

  let settings = $state<Settings>(DEFAULTS);
  let section = $state<SectionId>('appearance');
  let autostart = $state(false);
  let savedVisible = $state(false);
  let saveFailed = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let savedTimer: ReturnType<typeof setTimeout> | undefined;
  let saveDirty = false;
  let mainEl = $state<HTMLElement>();

  const integrations = $derived([
    {
      label: 'Notifikasi Windows',
      hint: 'Cerminkan toast Action Center',
      icon: 'bell' as IconName,
      on: settings.notifications,
      action: () => update({ notifications: !settings.notifications }, true),
    },
    {
      label: 'Mulai otomatis',
      hint: 'Jalankan Desk Island saat login',
      icon: 'bolt' as IconName,
      on: autostart,
      action: () => void toggleAutostart(),
    },
  ]);

  const poolLines = $derived(
    settings.greeting_pool
      .split('\n')
      .filter((line) => line.trim().length > 0),
  );

  // Nothing scrolls: the content sets its own height and the window is fitted to it,
  // so `main` must stay content-sized instead of stretching to the viewport.
  const MIN_HEIGHT = 420;

  async function fitWindow() {
    if (!mainEl) return;
    await document.fonts.ready;
    const wanted = Math.max(MIN_HEIGHT, Math.ceil(mainEl.getBoundingClientRect().height));
    const win = getCurrentWindow();
    const sf = await win.scaleFactor();
    const inner = await win.innerSize();
    const logical = inner.toLogical(sf);
    if (Math.abs(logical.height - wanted) <= 2) return;
    const outer = await win.outerPosition();
    const delta = wanted - logical.height;
    await win.setSize(new LogicalSize(logical.width, wanted));
    // Grow and shrink around the centre — `center()` would throw the window back to the
    // middle of the screen on every section switch.
    await win.setPosition(
      new PhysicalPosition(outer.x, Math.max(0, outer.y - Math.round((delta * sf) / 2))),
    );
  }

  onMount(() => {
    let unlistenAutostart: (() => void) | undefined;
    let unlistenResize: (() => void) | undefined;
    void getSettings().then((s) => {
      if (s) {
        settings = s;
        document.documentElement.dataset.theme = s.theme;
      }
      void fitWindow();
    });
    void getAutostart().then((v) => (autostart = v));
    void listen<boolean>('autostart-changed', (e) => (autostart = e.payload)).then((off) => {
      unlistenAutostart = off;
    });
    // Fit once after the first paint, before settings.json arrives, so a slow disk
    // read never leaves the window at its oversized creation height.
    requestAnimationFrame(() => void fitWindow());

    // Switching section, theme or window width rewraps the descriptions, so the content
    // height is re-measured rather than trusted once. The native `resized` hook covers
    // the user shrinking the window by hand: the height is restored, never clipped.
    const observer = new ResizeObserver(() => void fitWindow());
    if (mainEl) observer.observe(mainEl);
    void getCurrentWindow()
      .onResized(() => void fitWindow())
      .then((off) => {
        unlistenResize = off;
      });

    return () => {
      observer.disconnect();
      unlistenAutostart?.();
      unlistenResize?.();
      clearTimeout(savedTimer);
      clearTimeout(saveTimer);
      if (saveDirty) void persist();
    };
  });

  // Discrete controls (theme, anchor, idle variant, toggles) commit immediately so the
  // island reacts on the very next frame. Only the continuous sliders are debounced:
  // dragging fires oninput many times a second and each write hits settings.json.
  function persist() {
    saveDirty = false;
    return setSettings(settings).then(
      () => {
        saveFailed = false;
        savedVisible = true;
        clearTimeout(savedTimer);
        savedTimer = setTimeout(() => (savedVisible = false), 2000);
      },
      () => {
        // settings.json was not written; say so instead of looking saved. The value stays
        // in the widgets so a retry (or closing the window) re-sends the same settings.
        saveFailed = true;
      },
    );
  }

  function update(patch: Partial<Settings>, immediate = false) {
    settings = { ...settings, ...patch };
    document.documentElement.dataset.theme = settings.theme;
    saveDirty = true;
    clearTimeout(saveTimer);
    if (immediate) {
      void persist();
      return;
    }
    saveTimer = setTimeout(() => void persist(), 250);
  }

  // The OS title-bar ✕ (and Alt+F4) destroys the webview without running
  // closeSettings(), and a pending debounced persist would die with it. Range inputs
  // fire `change` on release, so committing there means there is never an unsaved
  // drag left when the window closes by any route.
  function flushSave() {
    clearTimeout(saveTimer);
    if (saveDirty) void persist();
  }

  async function toggleAutostart() {
    const actual = await setAutostart(!autostart);
    if (actual !== null) autostart = actual;
  }

  function clamp(value: number, min: number, max: number) {
    return Math.max(min, Math.min(max, value));
  }

  function seconds(ms: number) {
    if (ms < 1000) return `${ms} ms`;
    const value = ms / 1000;
    return `${Number.isInteger(value) ? value : value.toFixed(1).replace('.', ',')} s`;
  }

  function fill(value: number, min: number, max: number) {
    return `${((value - min) / (max - min)) * 100}%`;
  }
</script>

<main bind:this={mainEl} class="settings-shell px-4 py-3 text-[var(--ink)]">
  <div class="mx-auto w-full max-w-[860px]">
    <header class="settings-header mb-2 flex items-center gap-2.5">
      <span class="settings-mark" aria-hidden="true">
        <span class="settings-eye"><span class="settings-pupil"></span></span>
        <span class="settings-eye"><span class="settings-pupil"></span></span>
      </span>
      <div class="min-w-0 flex-1 leading-tight">
        <p class="text-[9px] font-medium tracking-[0.2em] text-[var(--faint)] uppercase">
          Desk Island
        </p>
        <h1 class="text-[15px] font-semibold tracking-tight">Setelan</h1>
      </div>
      <span
        class="saved-pill"
        class:saved-pill-show={savedVisible || saveFailed}
        class:saved-pill-error={saveFailed}
        role="status"
      >
        <span class="size-[5px] rounded-full bg-current"></span>
        {saveFailed ? 'Gagal menyimpan' : 'Tersimpan'}
      </span>
    </header>

    <div class="settings-body">
      <nav class="settings-nav" aria-label="Bagian setelan">
        {#each SECTIONS as item}
          <button
            type="button"
            class="nav-item"
            class:nav-item-on={section === item.id}
            aria-current={section === item.id ? 'page' : undefined}
            onclick={() => (section = item.id)}
          >
            <span class="nav-icon"><Icon name={item.icon} class="size-3.5" /></span>
            <span class="min-w-0 flex-1 leading-tight">
              <span class="nav-label">{item.label}</span>
              <span class="nav-hint">{item.hint}</span>
            </span>
          </button>
        {/each}
      </nav>

      <div class="settings-page">
        {#key section}
          <div class="page-in">
            {#if section === 'appearance'}
              <div class="page-head">
                <h2 class="page-title">Tampilan</h2>
                <p class="page-sub">Tema, posisi, navigasi, dan gaya pill saat menganggur</p>
              </div>

              <div class="fields">
                <div class="field">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="palette" class="size-3.5" /></span>
                    <span class="field-name">Tema</span>
                  </span>
                  <span class="field-control">
                    <div class="segmented" role="group" aria-label="Tema">
                      {#each THEME_OPTIONS as option}
                        <button
                          type="button"
                          class="segmented-item"
                          class:segmented-item-on={settings.theme === option.value}
                          aria-pressed={settings.theme === option.value}
                          onclick={() => update({ theme: option.value }, true)}
                        >
                          <Icon name={option.icon} class="size-3.5" />
                          <span>{option.label}</span>
                        </button>
                      {/each}
                    </div>
                  </span>
                  <span class="field-desc">Gelap atau terang</span>
                </div>

                <div class="field">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="align-center" class="size-3.5" /></span>
                    <span class="field-name">Posisi island</span>
                  </span>
                  <span class="field-control">
                    <div class="segmented" role="group" aria-label="Posisi island">
                      {#each ANCHOR_OPTIONS as option}
                        <button
                          type="button"
                          class="segmented-item"
                          class:segmented-item-on={settings.anchor === option.value}
                          aria-pressed={settings.anchor === option.value}
                          onclick={() => update({ anchor: option.value }, true)}
                        >
                          <Icon name={option.icon} class="size-3.5" />
                          <span>{option.label}</span>
                        </button>
                      {/each}
                    </div>
                  </span>
                  <span class="field-desc">Tepi acuan saat menempel</span>
                </div>

                <div class="field field-range">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="align-center" class="size-3.5" /></span>
                    <span class="field-name">Jarak dari tepi atas</span>
                  </span>
                  <span class="field-control">
                    <span class="value-chip">{settings.top_margin} px</span>
                  </span>
                  <span class="field-desc">0 menempel di tepi</span>
                  <input
                    class="range"
                    type="range"
                    min={0}
                    max={200}
                    step={2}
                    value={settings.top_margin}
                    aria-label="Jarak dari tepi atas"
                    style="--fill: {fill(settings.top_margin, 0, 200)}"
                    oninput={(e) =>
                      update({
                        top_margin: clamp(
                          Number((e.target as HTMLInputElement).value),
                          0,
                          200,
                        ),
                      })}
                    onchange={flushSave}
                  />
                </div>

                <div class="field">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="layout-rail" class="size-3.5" /></span>
                    <span class="field-name">Navigasi panel</span>
                  </span>
                  <span class="field-control">
                    <div class="segmented" role="group" aria-label="Navigasi panel">
                      {#each NAV_OPTIONS as option}
                        <button
                          type="button"
                          class="segmented-item"
                          class:segmented-item-on={settings.nav === option.value}
                          aria-pressed={settings.nav === option.value}
                          onclick={() => update({ nav: option.value }, true)}
                        >
                          <Icon name={option.icon} class="size-3.5" />
                          <span>{option.label}</span>
                        </button>
                      {/each}
                    </div>
                  </span>
                  <span class="field-desc">
                    {settings.nav === 'tabs' ? 'Tab di atas panel' : 'Rail ikon di kiri'}
                  </span>
                </div>

                <div class="field field-wide">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="eye" class="size-3.5" /></span>
                    <span class="field-name">Tampilan saat menganggur</span>
                  </span>
                  <span class="field-desc">
                    {settings.idle === 'auto' ? 'Berganti otomatis tiap 6 detik' : 'Tetap pada pilihan'}
                  </span>
                  <span class="field-control">
                    <div class="grid idle-grid grid-cols-4 gap-1.5">
                      {#each IDLE_OPTIONS as option}
                        <button
                          type="button"
                          class="idle-tile"
                          class:idle-tile-on={settings.idle === option.value}
                          aria-pressed={settings.idle === option.value}
                          onclick={() => update({ idle: option.value }, true)}
                        >
                          <span class="idle-tile-icon"><Icon name={option.icon} class="size-3" /></span>
                          <span class="idle-tile-label">{option.label}</span>
                          {#if settings.idle === option.value}<span class="idle-tile-dot"></span>{/if}
                        </button>
                      {/each}
                    </div>
                  </span>
                </div>

                <div class="field field-range">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="enter" class="size-3.5" /></span>
                    <span class="field-name">Radius pill</span>
                  </span>
                  <span class="field-control">
                    <span class="value-chip">{settings.radius_idle} px</span>
                  </span>
                  <span class="field-desc">Sudut saat menganggur, HUD, dan notifikasi</span>
                  <input
                    class="range"
                    type="range"
                    min={8}
                    max={25}
                    step={1}
                    value={settings.radius_idle}
                    aria-label="Radius pill"
                    style="--fill: {fill(settings.radius_idle, 8, 25)}"
                    oninput={(e) =>
                      update({
                        radius_idle: clamp(
                          Number((e.target as HTMLInputElement).value),
                          8,
                          25,
                        ),
                      })}
                    onchange={flushSave}
                  />
                </div>

                <div class="field field-range">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="enter" class="size-3.5" /></span>
                    <span class="field-name">Radius panel</span>
                  </span>
                  <span class="field-control">
                    <span class="value-chip">{settings.radius_panel} px</span>
                  </span>
                  <span class="field-desc">Sudut saat island terbuka penuh</span>
                  <input
                    class="range"
                    type="range"
                    min={8}
                    max={40}
                    step={1}
                    value={settings.radius_panel}
                    aria-label="Radius panel"
                    style="--fill: {fill(settings.radius_panel, 8, 40)}"
                    oninput={(e) =>
                      update({
                        radius_panel: clamp(
                          Number((e.target as HTMLInputElement).value),
                          8,
                          40,
                        ),
                      })}
                    onchange={flushSave}
                  />
                </div>
              </div>
            {:else if section === 'greeting'}
              <div class="page-head">
                <h2 class="page-title">Sambutan</h2>
                <p class="page-sub">Teks di pill saat baru menyala dan saat island kembali tampil</p>
              </div>

              <div class="greet-modes" role="group" aria-label="Sumber sambutan">
                {#each GREETING_OPTIONS as option}
                  {@const active = settings.greeting_mode === option.value}
                  <button
                    type="button"
                    class="greet-mode"
                    class:greet-mode-on={active}
                    aria-pressed={active}
                    onclick={() => update({ greeting_mode: option.value }, true)}
                  >
                    <span class="greet-mode-icon"><Icon name={option.icon} class="size-4" /></span>
                    <span class="min-w-0 flex-1 leading-tight">
                      <span class="greet-mode-label">{option.label}</span>
                      <span class="greet-mode-desc">{option.desc}</span>
                    </span>
                    <span class="greet-mode-check" aria-hidden="true">
                      {#if active}<Icon name="check" class="size-3.5" />{/if}
                    </span>
                  </button>
                {/each}
              </div>

              <div class="greet-cols">
                {#if settings.greeting_mode === 'custom'}
                  <div class="greet-editor">
                    <div class="greet-field">
                      <div class="greet-field-head">
                        <label class="greet-field-label" for="greet-line1">Baris utama</label>
                        <span class="greet-count">{settings.greeting_text.length}/60</span>
                      </div>
                      <input
                        id="greet-line1"
                        class="greeting-input"
                        type="text"
                        maxlength={60}
                        placeholder="Selamat datang"
                        value={settings.greeting_text}
                        oninput={(e) =>
                          update({ greeting_text: (e.target as HTMLInputElement).value })}
                        onchange={flushSave}
                      />
                    </div>

                    <div class="greet-field" class:greet-field-off={!settings.greeting_show2}>
                      <div class="greet-field-head">
                        <label class="greet-field-label" for="greet-line2">Baris kedua</label>
                        <span class="greet-field-head-actions">
                          <span class="greet-count">{settings.greeting_text2.length}/40</span>
                          <button
                            type="button"
                            class="toggle toggle-mini"
                            class:toggle-on={settings.greeting_show2}
                            role="switch"
                            aria-checked={settings.greeting_show2}
                            aria-label="Tampilkan baris kedua"
                            onclick={() => update({ greeting_show2: !settings.greeting_show2 }, true)}
                          >
                            <span class="toggle-knob"></span>
                          </button>
                        </span>
                      </div>
                      <input
                        id="greet-line2"
                        class="greeting-input"
                        type="text"
                        maxlength={40}
                        placeholder="Selamat datang kembali"
                        disabled={!settings.greeting_show2}
                        value={settings.greeting_text2}
                        oninput={(e) =>
                          update({ greeting_text2: (e.target as HTMLInputElement).value })}
                        onchange={flushSave}
                      />
                    </div>

                    <p class="greet-note">
                      Baris utama dikosongkan → kembali ke acak bawaan. Perubahan langsung
                      diputar ulang di pill.
                    </p>
                  </div>
                {:else}
                  <div class="greet-editor">
                    <div class="greet-field">
                      <div class="greet-field-head">
                        <span class="greet-field-label">Sambutan utama</span>
                        <span class="greet-count">bawaan</span>
                      </div>
                      <p class="greet-note">
                        Mode Bawaan memilih salah satu ucapan secara acak sesuai jam — selamat
                        pagi, siang, sore, atau malam. Pilih Kustom untuk menulis pesan sendiri.
                      </p>
                    </div>
                  </div>
                {/if}

                <div class="greet-editor">
                  <div class="greet-field">
                    <div class="greet-field-head">
                      <label class="greet-field-label" for="greet-pool">
                        Saat island muncul lagi
                      </label>
                      <span class="greet-count">{poolLines.length}/12</span>
                    </div>
                    <textarea
                      id="greet-pool"
                      class="greeting-input greeting-area"
                      rows={4}
                      placeholder={"Hai, balik lagi? | Lanjut semangat\nSelamat bekerja kembali"}
                      value={settings.greeting_pool}
                      oninput={(e) =>
                        update({ greeting_pool: (e.target as HTMLTextAreaElement).value })}
                      onchange={flushSave}
                    ></textarea>
                    <p class="greet-note">
                      Diputar bergantian tiap island kembali setelah disembunyikan. Satu baris
                      = satu ucapan, format <strong>judul | pesan</strong>.
                    </p>
                  </div>
                </div>
              </div>

              <div class="greet-presets">
                <span class="greet-presets-label">Opsi cepat</span>
                <div class="flex min-w-0 flex-1 flex-wrap gap-1.5">
                  {#each GREETING_PRESETS as preset}
                    <button
                      type="button"
                      class="preset-chip"
                      class:preset-chip-on={settings.greeting_text === preset}
                      onclick={() => update({ greeting_text: preset }, true)}
                    >
                      {preset}
                    </button>
                  {/each}
                </div>
              </div>

              <div class="fields">
                <div class="field field-range">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="eye" class="size-3.5" /></span>
                    <span class="field-name">Ukuran maskot</span>
                  </span>
                  <span class="field-control">
                    <span class="value-chip">{settings.mascot_scale}%</span>
                  </span>
                  <span class="field-desc">Badge mata pada pill</span>
                  <input
                    class="range"
                    type="range"
                    min={70}
                    max={140}
                    step={5}
                    value={settings.mascot_scale}
                    aria-label="Ukuran maskot"
                    style="--fill: {fill(settings.mascot_scale, 70, 140)}"
                    oninput={(e) =>
                      update({
                        mascot_scale: clamp(
                          Number((e.target as HTMLInputElement).value),
                          70,
                          140,
                        ),
                      })}
                    onchange={flushSave}
                  />
                </div>
                <div class="field field-range">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="timer" class="size-3.5" /></span>
                    <span class="field-name">Durasi sambutan</span>
                  </span>
                  <span class="field-control">
                    <span class="value-chip">{seconds(settings.welcome_ms)}</span>
                  </span>
                  <span class="field-desc">Terpisah dari durasi ganti varian</span>
                  <input
                    class="range"
                    type="range"
                    min={3500}
                    max={20000}
                    step={250}
                    value={settings.welcome_ms}
                    aria-label="Durasi sambutan"
                    style="--fill: {fill(settings.welcome_ms, 3500, 20000)}"
                    oninput={(e) =>
                      update({
                        welcome_ms: clamp(
                          Number((e.target as HTMLInputElement).value),
                          3500,
                          20000,
                        ),
                      })}
                    onchange={flushSave}
                  />
                </div>
              </div>
            {:else if section === 'behavior'}
              <div class="page-head">
                <h2 class="page-title">Perilaku</h2>
                <p class="page-sub">Durasi tampil dan ritme pembacaan data</p>
              </div>

              <div class="fields">
                {#each BEHAVIOR_ROWS as row}
                  <div class="field field-range">
                    <span class="field-label">
                      <span class="field-icon"><Icon name={row.icon} class="size-3.5" /></span>
                      <span class="field-name">{row.label}</span>
                    </span>
                    <span class="field-control">
                      <span class="value-chip">{seconds(settings[row.key])}</span>
                    </span>
                    <span class="field-desc">{row.hint}</span>
                    <input
                      class="range"
                      type="range"
                      min={row.min}
                      max={row.max}
                      step={row.step}
                      value={settings[row.key]}
                      aria-label={row.label}
                      style="--fill: {fill(settings[row.key], row.min, row.max)}"
                      oninput={(e) =>
                        update({
                          [row.key]: clamp(
                            Number((e.target as HTMLInputElement).value),
                            row.min,
                            row.max,
                          ),
                        } as Partial<Settings>)}
                      onchange={flushSave}
                    />
                  </div>
                {/each}
              </div>
            {:else}
              <div class="page-head">
                <h2 class="page-title">Integrasi</h2>
                <p class="page-sub">Sambungkan pill dengan Windows</p>
              </div>

              <div class="fields">
                {#each integrations as row}
                  <div class="field">
                    <span class="field-label">
                      <span class="field-icon"><Icon name={row.icon} class="size-3.5" /></span>
                      <span class="field-name">{row.label}</span>
                    </span>
                    <span class="field-control">
                      <button
                        type="button"
                        role="switch"
                        aria-checked={row.on}
                        aria-label={row.label}
                        class="toggle"
                        class:toggle-on={row.on}
                        onclick={row.action}
                      >
                        <span class="toggle-knob"></span>
                      </button>
                    </span>
                    <span class="field-desc">{row.hint}</span>
                  </div>
                {/each}
                <div class="field field-wide">
                  <span class="field-label">
                    <span class="field-icon"><Icon name="keyboard" class="size-3.5" /></span>
                    <span class="field-name">Pintasan panel</span>
                  </span>
                  <span class="field-desc">Buka dan pin panel dari mana saja</span>
                  <span class="field-control">
                    <span class="keycaps" aria-label="Ctrl Alt Space">
                      <kbd>Ctrl</kbd>
                      <kbd>Alt</kbd>
                      <kbd>Space</kbd>
                    </span>
                  </span>
                </div>
              </div>

              <p class="integration-foot">Setelan disimpan otomatis ke perangkat ini.</p>
            {/if}
          </div>
        {/key}
      </div>
    </div>
  </div>
</main>

<style>
  .settings-header {
    animation: settings-rise 420ms var(--ease-out) both;
  }

  /* Height comes from the content; there is no fixed box with a scrollable page. */
  .settings-body {
    display: grid;
    grid-template-columns: 152px minmax(0, 1fr);
    min-height: 336px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--ink) 4%, transparent), transparent 42%),
      var(--card);
    box-shadow:
      0 8px 22px rgb(0 0 0 / 0.14),
      inset 0 1px 0 var(--sheen);
    overflow: hidden;
    animation: settings-rise 480ms var(--ease-out) both;
  }

  :global(:root[data-theme='light']) .settings-body {
    box-shadow:
      0 8px 20px rgb(15 23 42 / 0.06),
      inset 0 1px 0 rgb(255 255 255 / 0.85);
  }

  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 7px;
    border-right: 1px solid var(--line);
    background: color-mix(in srgb, var(--ink) 3%, transparent);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 5px 6px;
    border-radius: 10px;
    color: var(--muted);
    text-align: left;
    transition:
      color 160ms ease,
      background 160ms ease,
      box-shadow 160ms ease;
  }

  .nav-item:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--ink);
  }

  .nav-item-on,
  .nav-item-on:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent-ink);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .nav-icon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    border-radius: 8px;
    background: color-mix(in srgb, var(--ink) 5%, transparent);
    transition: background 160ms ease;
  }

  .nav-item-on .nav-icon {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }

  .nav-label {
    display: block;
    overflow: hidden;
    font-size: 11.5px;
    font-weight: 600;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .nav-hint {
    display: block;
    overflow: hidden;
    margin-top: 0;
    font-size: 9.5px;
    font-weight: 400;
    line-height: 1.2;
    color: var(--faint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The body is at least tall enough for the longest page, so `justify-content` only
     ever distributes slack: a short page centres, a tall one stays top-aligned. */
  .settings-page {
    display: flex;
    min-width: 0;
    flex-direction: column;
    justify-content: center;
    padding: 11px 12px;
  }

  .page-head {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .page-title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.25;
  }

  .page-sub {
    font-size: 10.5px;
    line-height: 1.3;
    color: var(--faint);
  }

  .page-in {
    animation: page-in 220ms var(--ease-out) both;
  }

  @keyframes page-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .settings-mark {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    width: 32px;
    height: 32px;
    border: 1px solid color-mix(in srgb, var(--di-blue) 32%, transparent);
    border-radius: 11px;
    background: linear-gradient(
      145deg,
      color-mix(in srgb, var(--di-blue) 30%, transparent),
      color-mix(in srgb, var(--di-purple) 16%, transparent)
    );
    box-shadow:
      0 0 14px color-mix(in srgb, var(--di-blue) 22%, transparent),
      inset 0 1px 0 var(--sheen);
  }

  .settings-eye {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 9px;
    height: 13px;
    border-radius: 4px;
    background: #fff;
    box-shadow: inset 0 -1px 2px rgb(15 23 42 / 0.25);
  }

  .settings-pupil {
    width: 3.5px;
    height: 6px;
    border-radius: 999px;
    background: #0b0d12;
  }

  .saved-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    border: 1px solid color-mix(in srgb, var(--di-green) 26%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--di-green) 12%, transparent);
    color: var(--di-green);
    font-size: 10.5px;
    font-weight: 500;
    opacity: 0;
    transform: translateY(2px) scale(0.96);
    transition:
      opacity 200ms ease,
      transform 240ms var(--ease-out);
    pointer-events: none;
  }

  .saved-pill-show {
    opacity: 1;
    transform: none;
  }

  .saved-pill-error {
    border-color: color-mix(in srgb, var(--di-red) 34%, transparent);
    background: color-mix(in srgb, var(--di-red) 14%, transparent);
    color: var(--di-red);
  }

  /* Two columns of self-sizing cards. `minmax(0, …)` keeps a long label or control from
     widening the track, which is what would otherwise push the page sideways. */
  .fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-top: 10px;
  }

  .field {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 8px;
    row-gap: 2px;
    padding: 7px 9px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: color-mix(in srgb, var(--ink) 2%, transparent);
  }

  .field-label {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 6px;
    grid-column: 1;
    grid-row: 1;
  }

  .field-control {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    grid-column: 2;
    grid-row: 1 / span 2;
  }

  .field-desc {
    min-width: 0;
    grid-column: 1;
    grid-row: 2;
    font-size: 10px;
    line-height: 1.3;
    color: var(--faint);
  }

  /* Controls that need the whole card width: the idle picker, the keycap hint. */
  .field-wide .field-desc {
    grid-column: 1 / -1;
  }

  .field-wide .field-control {
    grid-column: 1 / -1;
    grid-row: 3;
    justify-content: flex-start;
  }

  /* Slider cards keep the value chip beside the label and drop the track underneath. */
  .field-range .field-control {
    grid-row: 1;
  }

  .field-range .range {
    grid-column: 1 / -1;
    grid-row: 3;
  }

  .field-icon {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    border-radius: 6px;
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--muted);
  }

  .field-name {
    overflow: hidden;
    min-width: 0;
    font-size: 11.5px;
    font-weight: 600;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .idle-grid {
    width: 100%;
  }

  .segmented {
    display: flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
    border-radius: 10px;
    background: var(--track);
  }

  .segmented-item {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 24px;
    padding: 0 6px;
    border-radius: 8px;
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
    transition:
      color 160ms ease,
      background 160ms ease,
      box-shadow 160ms ease;
  }

  .segmented-item:hover {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 5%, transparent);
  }

  .segmented-item-on,
  .segmented-item-on:hover {
    color: var(--accent-ink);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 34%, transparent),
      0 2px 8px color-mix(in srgb, var(--accent) 12%, transparent);
    font-weight: 600;
  }

  .idle-tile {
    position: relative;
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 5px;
    padding: 4px 6px 4px 5px;
    border: 1px solid transparent;
    border-radius: 9px;
    background: color-mix(in srgb, var(--ink) 3%, transparent);
    color: var(--muted);
    transition:
      color 160ms ease,
      background 160ms ease,
      border-color 160ms ease,
      box-shadow 160ms ease;
  }

  .idle-tile:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--ink);
  }

  .idle-tile-on,
  .idle-tile-on:hover {
    border-color: color-mix(in srgb, var(--accent) 32%, transparent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent-ink);
    box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .idle-tile-icon {
    display: grid;
    place-items: center;
    width: 19px;
    height: 19px;
    flex-shrink: 0;
    border-radius: 6px;
    background: color-mix(in srgb, var(--ink) 5%, transparent);
    transition: background 160ms ease;
  }

  .idle-tile-on .idle-tile-icon {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }

  .idle-tile-label {
    overflow: hidden;
    min-width: 0;
    flex: 1;
    font-size: 10px;
    font-weight: 500;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .idle-tile-dot {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }

  .value-chip {
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--track);
    color: var(--ink);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .range {
    appearance: none;
    width: 100%;
    height: 16px;
    margin-top: 3px;
    background: transparent;
    cursor: pointer;
  }

  .range::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 999px;
    background: linear-gradient(
      to right,
      var(--accent) 0%,
      var(--accent) var(--fill, 50%),
      var(--track) var(--fill, 50%),
      var(--track) 100%
    );
  }

  .range::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border: 2px solid var(--accent);
    border-radius: 50%;
    background: var(--card);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.35);
    transition:
      transform 140ms var(--ease-out),
      box-shadow 140ms ease;
  }

  .range:hover::-webkit-slider-thumb {
    transform: scale(1.12);
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent),
      0 1px 4px rgb(0 0 0 / 0.35);
  }

  .range:focus-visible {
    outline: none;
  }

  .range:focus-visible::-webkit-slider-thumb {
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--accent) 24%, transparent),
      0 1px 4px rgb(0 0 0 / 0.35);
  }

  .range::-moz-range-track {
    height: 4px;
    border-radius: 999px;
    background: var(--track);
  }

  .range::-moz-range-progress {
    height: 4px;
    border-radius: 999px;
    background: var(--accent);
  }

  .range::-moz-range-thumb {
    width: 10px;
    height: 10px;
    border: 2px solid var(--accent);
    border-radius: 50%;
    background: var(--card);
  }

  .toggle {
    position: relative;
    width: 38px;
    height: 22px;
    flex-shrink: 0;
    border: 1px solid color-mix(in srgb, var(--line) 80%, transparent);
    border-radius: 999px;
    background: var(--track);
    transition:
      background 180ms ease,
      border-color 180ms ease,
      box-shadow 180ms ease;
  }

  .toggle-on {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 78%, transparent);
    box-shadow: 0 0 14px color-mix(in srgb, var(--accent) 24%, transparent);
  }

  .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: #fff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.3);
    transition: transform 200ms var(--ease-pop);
  }

  .toggle-on .toggle-knob {
    transform: translateX(16px);
  }

  /* Compact variant used inside the greeting editor's field header. */
  .toggle.toggle-mini {
    width: 30px;
    height: 17px;
  }

  .toggle-mini .toggle-knob {
    width: 11px;
    height: 11px;
  }

  .toggle-mini.toggle-on .toggle-knob {
    transform: translateX(13px);
  }

  .keycaps {
    display: flex;
    flex-shrink: 0;
    gap: 4px;
  }

  .keycaps kbd {
    padding: 3px 5px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    box-shadow: 0 1px 0 color-mix(in srgb, var(--line) 80%, transparent);
    color: var(--muted);
    font-family: inherit;
    font-size: 9.5px;
    font-weight: 500;
    line-height: 1;
  }

  .integration-foot {
    margin-top: 8px;
    padding-top: 7px;
    border-top: 1px solid var(--line);
    font-size: 10px;
    color: var(--faint);
  }

  .greeting-input {
    width: 100%;
    padding: 6px 10px;
    border: 1px solid color-mix(in srgb, var(--line) 80%, transparent);
    border-radius: 10px;
    background: var(--track);
    color: var(--ink);
    font-size: 12px;
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }

  .greeting-input::placeholder {
    color: var(--faint);
  }

  /* No drag handle: growing the textarea would push the page past the fitted window. */
  .greeting-area {
    line-height: 1.5;
    font-family: inherit;
    resize: none;
  }

  .greeting-input:focus {
    outline: none;
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .preset-chip {
    padding: 3px 9px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: color-mix(in srgb, var(--ink) 4%, transparent);
    color: var(--muted);
    font-size: 10.5px;
    white-space: nowrap;
    transition:
      color 160ms ease,
      background 160ms ease,
      border-color 160ms ease;
  }

  .preset-chip:hover {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }

  .preset-chip-on,
  .preset-chip-on:hover {
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent-ink);
    font-weight: 600;
  }

  /* Sambutan page: mode cards on top, the two editors side by side below them. */
  .greet-modes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-top: 10px;
  }

  .greet-cols {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    align-items: stretch;
    margin-top: 8px;
  }

  .greet-presets {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-top: 8px;
  }

  .greet-presets-label {
    flex-shrink: 0;
    padding-top: 4px;
    font-size: 10.5px;
    font-weight: 500;
    color: var(--muted);
  }

  .greet-mode {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 9px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: color-mix(in srgb, var(--ink) 3%, transparent);
    color: var(--muted);
    text-align: left;
    transition:
      color 160ms ease,
      background 160ms ease,
      border-color 160ms ease,
      box-shadow 160ms ease;
  }

  .greet-mode:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--ink);
  }

  .greet-mode-on,
  .greet-mode-on:hover {
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent-ink);
    box-shadow: 0 2px 12px color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .greet-mode-icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    border-radius: 8px;
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--muted);
    transition:
      background 160ms ease,
      color 160ms ease;
  }

  .greet-mode-on .greet-mode-icon {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent-ink);
  }

  .greet-mode-label {
    display: block;
    overflow: hidden;
    font-size: 12px;
    font-weight: 600;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .greet-mode-desc {
    display: block;
    overflow: hidden;
    margin-top: 1px;
    font-size: 9.5px;
    line-height: 1.25;
    color: var(--faint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .greet-mode-on .greet-mode-desc {
    color: color-mix(in srgb, var(--accent-ink) 72%, transparent);
  }

  .greet-mode-check {
    display: grid;
    place-items: center;
    width: 17px;
    height: 17px;
    flex-shrink: 0;
    border: 1px solid var(--line);
    border-radius: 999px;
    color: var(--accent-ink);
    transition:
      background 160ms ease,
      border-color 160ms ease;
  }

  .greet-mode-on .greet-mode-check {
    border-color: var(--accent);
    background: var(--accent);
  }

  .greet-editor {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 7px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 13px;
    background: color-mix(in srgb, var(--ink) 2%, transparent);
  }

  .greet-field {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 3px;
  }

  .greet-field-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .greet-field-head-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .greet-field-off .greet-field-label,
  .greet-field-off .greet-count {
    opacity: 0.5;
  }

  .greet-field-off .greeting-input {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .greet-field-label {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--muted);
  }

  .greet-count {
    font-size: 9.5px;
    font-variant-numeric: tabular-nums;
    color: var(--faint);
    white-space: nowrap;
  }

  .greet-note {
    font-size: 10px;
    line-height: 1.35;
    color: var(--faint);
  }

  @keyframes settings-rise {
    from {
      opacity: 0;
      transform: translateY(7px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .settings-header,
    .settings-body,
    .page-in {
      animation: none;
    }

    .saved-pill,
    .toggle,
    .toggle-knob,
    .segmented-item,
    .nav-item,
    .idle-tile,
    .preset-chip,
    .range::-webkit-slider-thumb {
      transition: none;
    }
  }
</style>
