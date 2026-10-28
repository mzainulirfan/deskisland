<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { previewReminder, saveReminders } from '$lib/services/desktop';
  import type { Reminder } from '$lib/types/system';

  const DAY_SHORT = ['Sen', 'Sel', 'Rab', 'Kam', 'Jum', 'Sab', 'Min'];
  const EVERY_DAY = 0b111_1111;
  /** Senin sampai Jumat: what a workday alarm is worth. */
  const WORKDAYS = 0b001_1111;
  const MAX = 12;
  const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;

  const reminders = $derived(island.reminders);
  const full = $derived(reminders.length >= MAX);

  function blank(): Reminder {
    return { id: 0, text: '', time: nextMinute(), days: EVERY_DAY, enabled: true };
  }

  function nextMinute() {
    const at = new Date(Date.now() + 60_000);
    const pad = (value: number) => String(value).padStart(2, '0');
    return `${pad(at.getHours())}:${pad(at.getMinutes())}`;
  }

  /** `id: 0` means the editor is writing a row that does not exist yet; the backend hands
   * out the real id on save. The editor never closes, so the draft is never null. */
  let draft = $state<Reminder>(blank());
  let busy = $state(false);

  const editing = $derived(draft.id !== 0);
  const timeOk = $derived(TIME_RE.test(draft.time));
  const textOk = $derived(draft.text.trim().length > 0);
  const canSave = $derived(timeOk && textOk && !busy && (!full || editing));

  function daysLabel(days: number) {
    if (days === EVERY_DAY) return 'Setiap hari';
    if (days === WORKDAYS) return 'Hari kerja';
    const named = DAY_SHORT.filter((_, index) => days & (1 << index));
    return named.length > 3 ? `${named.length} hari seminggu` : named.join(', ');
  }

  /** Optimistic like the favorites list: the store moves first, then the file. The reply is
   * the list the backend kept, so a newly created row picks up the id it was given. A failed
   * write rolls the store back so the list matches reminders.json. */
  let failure = $state('');

  async function commit(next: Reminder[]) {
    if (busy) return;
    busy = true;
    const previous = island.reminders;
    island.reminders = next;
    try {
      island.reminders = await saveReminders(next);
      failure = '';
    } catch {
      island.reminders = previous;
      failure = 'Gagal menyimpan ke berkas pengingat.';
    } finally {
      busy = false;
    }
  }

  function edit(reminder: Reminder) {
    draft = { ...reminder };
  }

  function reset() {
    draft = blank();
  }

  function save() {
    if (!canSave) return;
    const entry: Reminder = { ...draft, text: draft.text.trim() };
    void commit(
      entry.id ? reminders.map((r) => (r.id === entry.id ? entry : r)) : [...reminders, entry],
    );
    reset();
  }

  function remove(reminder: Reminder) {
    void commit(reminders.filter((r) => r.id !== reminder.id));
    if (draft.id === reminder.id) reset();
  }

  function toggle(reminder: Reminder) {
    void commit(
      reminders.map((r) => (r.id === reminder.id ? { ...r, enabled: !r.enabled } : r)),
    );
  }

  function setDay(bit: number) {
    const days = draft.days ^ bit;
    // An empty week is not a schedule; keep the day that was just switched off.
    draft.days = days === 0 ? EVERY_DAY : days;
  }

  function setDays(days: number) {
    draft.days = days;
  }
</script>

<div
  class="remind-shell flex h-full min-h-0 gap-2.5 rounded-[15px] p-2.5 ring-1 ring-[color:var(--line)]"
>
  <!-- ============ LEFT: the schedule ============ -->
  <div class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-6 shrink-0 items-center gap-1.5">
      <span class="size-1.5 rounded-full bg-[var(--di-amber)]" aria-hidden="true"></span>
      <h2
        class="text-[9px] font-medium uppercase tracking-[0.08em] text-[var(--muted)]"
      >
        Pengingat harian
      </h2>
      <span class="text-[9px] text-[var(--faint)]">{reminders.length} / {MAX}</span>
      {#if editing || draft.text}
        <button type="button" class="head-btn" onclick={reset}>
          <Icon name="plus" class="size-2.5" />
          Baru
        </button>
      {/if}
    </header>

    {#if reminders.length}
      <ul class="remind-list mt-1 min-h-0 flex-1" aria-label="Daftar pengingat">
        {#each reminders as reminder (reminder.id)}
          {@const active = draft.id === reminder.id}
          <li class="remind-row" class:remind-off={!reminder.enabled} class:remind-on={active}>
            <button
              type="button"
              class="remind-main"
              title="Ubah {reminder.text}"
              onclick={() => edit(reminder)}
            >
              <span class="remind-time">{reminder.time}</span>
              <span class="flex min-w-0 flex-col leading-tight">
                <span class="truncate text-[11px] font-semibold">{reminder.text}</span>
                <span class="truncate text-[9px] text-[var(--faint)]"
                  >{daysLabel(reminder.days)}</span
                >
              </span>
            </button>
            <span class="remind-actions">
              <button
                type="button"
                class="row-btn"
                title="Bunyikan sekarang"
                aria-label={`Uji ${reminder.text}`}
                onclick={() => void previewReminder(reminder.id)}
              >
                <Icon name="play" class="size-3" />
              </button>
              <button
                type="button"
                class="row-btn row-btn-x"
                title="Hapus"
                aria-label={`Hapus ${reminder.text}`}
                onclick={() => remove(reminder)}
              >
                <Icon name="trash" class="size-3.5" />
              </button>
            </span>
            <button
              type="button"
              class="remind-switch"
              class:remind-switch-on={reminder.enabled}
              role="switch"
              aria-checked={reminder.enabled}
              aria-label={reminder.enabled ? `Matikan ${reminder.text}` : `Nyalakan ${reminder.text}`}
              title={reminder.enabled ? 'Nyala' : 'Mati'}
              onclick={() => toggle(reminder)}
            >
              <span class="remind-knob"></span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="remind-empty">
        <span class="remind-empty-icon"><Icon name="bell" class="size-5" /></span>
        <p class="text-[10.5px] leading-snug text-[var(--muted)]">
          Belum ada pengingat.<br />Tulis yang pertama di kolom kanan.
        </p>
      </div>
    {/if}
    {#if failure}
      <p class="remind-failure">{failure}</p>
    {/if}
  </div>

  <!-- ============ RIGHT: the editor, always open ============ -->
  <div class="remind-editor flex w-[214px] shrink-0 flex-col">
    <p class="editor-title">
      {editing ? 'Ubah pengingat' : 'Pengingat baru'}
      {#if editing}
        <span class="editor-title-for truncate">{draft.text || 'tanpa nama'}</span>
      {/if}
    </p>

    <input
      class="remind-input"
      type="text"
      maxlength="40"
      placeholder="Minum air"
      aria-label="Isi pengingat"
      bind:value={draft.text}
      onkeydown={(event) => {
        if (event.key === 'Enter') save();
        if (event.key === 'Escape') reset();
      }}
    />

    <input
      class="remind-input remind-time-input mt-1.5"
      type="time"
      aria-label="Jam"
      bind:value={draft.time}
      onkeydown={(event) => {
        if (event.key === 'Enter') save();
        if (event.key === 'Escape') reset();
      }}
    />

    <div class="mt-1.5 flex gap-1">
      <button
        type="button"
        class="preset-btn"
        class:preset-btn-on={draft.days === EVERY_DAY}
        onclick={() => setDays(EVERY_DAY)}
      >
        Setiap hari
      </button>
      <button
        type="button"
        class="preset-btn"
        class:preset-btn-on={draft.days === WORKDAYS}
        onclick={() => setDays(WORKDAYS)}
      >
        Hari kerja
      </button>
    </div>

    <div class="mt-1.5 flex gap-0.5">
      {#each DAY_SHORT as label, index}
        {@const on = draft.days & (1 << index)}
        <button
          type="button"
          class="day-chip"
          class:day-chip-on={!!on}
          aria-pressed={!!on}
          aria-label={label}
          onclick={() => setDay(1 << index)}
        >
          {label}
        </button>
      {/each}
    </div>

    <div class="mt-auto flex items-center gap-2 pt-2">
      <p class="editor-hint">
        {full && !editing
          ? `Maksimal ${MAX} pengingat.`
          : timeOk && textOk
            ? `Berbunyi ${draft.time}.`
            : 'Isi teks dan jam (HH:MM).'}
      </p>
      <button type="button" class="save-btn" disabled={!canSave} onclick={save}>
        <Icon name="check" class="size-3.5" />
        Simpan
      </button>
    </div>
  </div>
</div>

<style>
  .remind-shell {
    --tint: var(--di-amber);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--tint) 7%, transparent), transparent 55%),
      var(--card);
  }

  .head-btn {
    display: flex;
    height: 18px;
    margin-left: auto;
    align-items: center;
    gap: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--di-amber) 14%, transparent);
    padding: 0 8px;
    color: var(--di-amber);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 26%, transparent);
    font-size: 9px;
    font-weight: 600;
    transition: background 120ms ease;
  }

  .head-btn:hover {
    background: color-mix(in srgb, var(--di-amber) 24%, transparent);
  }

  .remind-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--ink) 18%, transparent) transparent;
  }

  .remind-row {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 4px;
    height: 34px;
    padding-right: 4px;
    border-radius: 9px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow 120ms ease;
  }

  .remind-on {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 50%, transparent),
      0 1px 6px color-mix(in srgb, var(--ink) 10%, transparent);
  }

  .remind-off .remind-time,
  .remind-off .remind-main {
    opacity: 0.45;
  }

  .remind-main {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: 9px;
    height: 100%;
    padding: 0 4px 0 8px;
    text-align: left;
  }

  .remind-time {
    font-size: 12.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--di-amber);
  }

  /* Test and delete only matter while the pointer is on the row, so they stay out of the
   * way until then; the on/off switch is state, so it is always visible. */
  .remind-actions {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .remind-row:hover .remind-actions,
  .remind-row:focus-within .remind-actions {
    opacity: 1;
  }

  .row-btn {
    display: flex;
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    color: var(--muted);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .row-btn:hover {
    background: color-mix(in srgb, var(--ink) 8%, transparent);
    color: var(--ink);
  }

  .row-btn-x:hover {
    color: var(--di-red);
    background: color-mix(in srgb, var(--di-red) 12%, transparent);
  }

  .remind-switch {
    display: flex;
    width: 24px;
    height: 14px;
    flex-shrink: 0;
    align-items: center;
    border-radius: 999px;
    background: color-mix(in srgb, var(--ink) 12%, transparent);
    box-shadow: inset 0 0 0 1px var(--line);
    padding: 1px;
    transition: background 140ms ease;
  }

  .remind-switch-on {
    background: color-mix(in srgb, var(--di-amber) 55%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 70%, transparent);
  }

  .remind-knob {
    width: 12px;
    height: 12px;
    border-radius: 999px;
    background: var(--muted);
    transition:
      translate 140ms var(--ease-out),
      background 140ms ease;
  }

  .remind-switch-on .remind-knob {
    translate: 10px 0;
    background: var(--di-amber);
  }

  .remind-empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    text-align: center;
  }

  .remind-empty-icon {
    display: flex;
    width: 40px;
    height: 40px;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    color: var(--di-amber);
    background: color-mix(in srgb, var(--di-amber) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 22%, transparent);
  }

  .remind-failure {
    flex-shrink: 0;
    margin-top: 4px;
    font-size: 9px;
    font-weight: 600;
    color: var(--di-red);
  }

  /* ---- editor column ---- */
  .remind-editor {
    padding-left: 10px;
    border-left: 1px solid var(--line);
  }

  .editor-title {
    display: flex;
    min-width: 0;
    align-items: baseline;
    gap: 5px;
    margin-bottom: 6px;
    color: var(--muted);
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .editor-title-for {
    min-width: 0;
    color: var(--faint);
    font-weight: 500;
    text-transform: none;
    letter-spacing: normal;
  }

  .remind-input {
    width: 100%;
    height: 26px;
    padding: 0 9px;
    border: 0;
    border-radius: 8px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }

  .remind-input:focus {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 55%, transparent),
      0 0 0 3px color-mix(in srgb, var(--di-amber) 12%, transparent);
    outline: none;
  }

  .remind-input::placeholder {
    color: var(--faint);
  }

  .remind-time-input {
    height: 26px;
    color: var(--di-amber);
    font-size: 12.5px;
    font-weight: 700;
  }

  .preset-btn {
    min-width: 0;
    flex: 1;
    height: 26px;
    border-radius: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 9px;
    font-weight: 600;
    white-space: nowrap;
    transition:
      background 120ms ease,
      color 120ms ease;
  }

  .preset-btn:hover {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }

  .preset-btn-on,
  .preset-btn-on:hover {
    background: color-mix(in srgb, var(--di-amber) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 34%, transparent);
    color: var(--di-amber);
  }

  .day-chip {
    min-width: 0;
    flex: 1;
    height: 20px;
    border-radius: 6px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--faint);
    font-size: 8.5px;
    font-weight: 600;
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .day-chip:hover {
    color: var(--ink);
  }

  .day-chip-on,
  .day-chip-on:hover {
    background: color-mix(in srgb, var(--di-amber) 22%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 40%, transparent);
    color: var(--ink);
  }

  .editor-hint {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--faint);
    font-size: 8.5px;
    line-height: 1.25;
  }

  .save-btn {
    display: flex;
    height: 26px;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--di-amber) 82%, transparent);
    padding: 0 11px;
    color: #0b0d12;
    font-size: 10.5px;
    font-weight: 700;
    transition:
      opacity 120ms var(--ease-out),
      filter 120ms ease;
  }

  .save-btn:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .save-btn:disabled {
    opacity: 0.4;
  }
</style>
