<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { getAppIcon, launchApp, listAppShortcuts, setFavorites } from '$lib/services/desktop';
  import type { Favorite } from '$lib/types/system';

  const TONES = ['var(--di-blue)', 'var(--di-purple)', 'var(--di-green)', 'var(--di-amber)', 'var(--di-red)'];

  let picking = $state(false);
  let query = $state('');

  const favorites = $derived(island.favorites);
  const results = $derived(
    island.appShortcuts
      .filter((app) => app.name.toLowerCase().includes(query.trim().toLowerCase()))
      .slice(0, 40),
  );
  const favoritePaths = $derived(new Set(favorites.map((item) => item.path)));

  // Icons are 48-px PNG reads that cost a GDI round-trip each, so only the paths a
  // user can actually see are fetched: favorite tiles, plus the visible picker rows.
  const inFlight = new Set<string>();
  $effect(() => {
    const paths = picking ? results.map((app) => app.path) : favorites.map((item) => item.path);
    for (const path of paths) {
      if (path in island.appIcons || inFlight.has(path)) continue;
      inFlight.add(path);
      void getAppIcon(path).then((url) => {
        inFlight.delete(path);
        if (url) island.appIcons[path] = url;
      });
    }
  });

  async function openPicker() {
    picking = true;
    query = '';
    if (island.appShortcuts.length === 0) {
      island.appShortcuts = await listAppShortcuts();
    }
  }

  function commit(next: Favorite[]) {
    const sorted = next
      .slice()
      .sort((a, b) => a.name.localeCompare(b.name, 'id', { sensitivity: 'base' }));
    island.favorites = sorted;
    void setFavorites(sorted);
  }

  const full = $derived(favorites.length >= 12);

  function toggle(app: Favorite) {
    if (favoritePaths.has(app.path)) {
      commit(favorites.filter((item) => item.path !== app.path));
    } else if (!full) {
      commit([...favorites, app]);
    }
  }

  function remove(app: Favorite) {
    commit(favorites.filter((item) => item.path !== app.path));
  }

  // Right-click a tile to remove it without entering edit mode.
  function onContext(event: MouseEvent, app: Favorite) {
    event.preventDefault();
    remove(app);
  }
</script>

<div
  class="flex h-full flex-col rounded-[16px] bg-[var(--card)] p-3 ring-1 ring-[color:var(--line)]"
>
  {#if !picking}
    <header class="flex h-5 shrink-0 items-center justify-between gap-3">
      <h2
        class="flex items-center gap-1.5 text-[9px] font-medium tracking-[0.08em] text-[var(--muted)] uppercase"
      >
        <span class="size-1.5 rounded-full bg-[var(--di-blue)]"></span>
        Aplikasi
        <span class="normal-case text-[9px] font-normal tracking-normal text-[var(--faint)]"
          >{favorites.length}/12</span
        >
      </h2>
      <button
        type="button"
        class="head-btn"
        aria-label="Atur aplikasi favorit"
        title="Atur aplikasi favorit"
        onclick={() => void openPicker()}
      >
        <Icon name="plus" class="size-2.5" />
        Atur
      </button>
    </header>

    {#if favorites.length}
      <ul
        class="mt-2 grid min-h-0 flex-1 grid-cols-4 content-center gap-x-2 gap-y-2"
        aria-label="Aplikasi favorit"
      >
        {#each favorites as app, index (app.path)}
          <li class="app-cell" style:--tile-tone={TONES[index % TONES.length]}>
            <button
              type="button"
              class="app-tile"
              title="{app.name} — klik untuk membuka · klik kanan untuk menghapus"
              onclick={() => void launchApp(app.path)}
              oncontextmenu={(event) => onContext(event, app)}
            >
              <span class="app-badge">
                {#if island.appIcons[app.path]}
                  <img class="app-icon" src={island.appIcons[app.path]} alt="" draggable="false" />
                {:else}
                  <span class="app-initial">{app.name.charAt(0).toUpperCase()}</span>
                {/if}
              </span>
              <span class="app-name">{app.name}</span>
            </button>
            <button
              type="button"
              class="app-remove"
              aria-label="Hapus {app.name} dari favorit"
              title="Hapus dari favorit"
              onclick={() => remove(app)}
            >
              <Icon name="close" class="size-2" />
            </button>
          </li>
        {/each}
        {#if !full}
          <li class="app-cell">
            <button
              type="button"
              class="app-tile app-add"
              aria-label="Tambah aplikasi favorit"
              title="Tambah aplikasi"
              onclick={() => void openPicker()}
            >
              <span class="app-badge"><Icon name="plus" class="size-4" /></span>
              <span class="app-name">Tambah</span>
            </button>
          </li>
        {/if}
      </ul>
    {:else}
      <div class="app-empty">
        <span class="app-empty-icon"><Icon name="apps" class="size-5" /></span>
        <p class="text-[10.5px] leading-snug text-[var(--muted)]">
          Belum ada aplikasi favorit.<br />Pilih pintasan dari Start Menu.
        </p>
        <button type="button" class="app-empty-cta" onclick={() => void openPicker()}>
          <Icon name="plus" class="size-3" />
          Tambah aplikasi
        </button>
      </div>
    {/if}
  {:else}
    <header class="flex h-6 shrink-0 items-center gap-2">
      <input
        class="picker-search"
        type="text"
        placeholder="Cari aplikasi…"
        bind:value={query}
        aria-label="Cari aplikasi"
      />
      <span class="picker-count tabular-nums">{favorites.length}/12</span>
      <button type="button" class="picker-done" onclick={() => (picking = false)}>
        <Icon name="check" class="size-3" />
        Selesai
      </button>
    </header>

    <ul class="picker-list mt-1.5 min-h-0 flex-1 overflow-y-auto pr-1" aria-label="Daftar aplikasi">
      {#each results as app (app.path)}
        {@const chosen = favoritePaths.has(app.path)}
        {@const blocked = !chosen && full}
        <li>
          <button
            type="button"
            class="picker-row"
            class:picker-row-on={chosen}
            class:picker-row-blocked={blocked}
            disabled={blocked}
            aria-pressed={chosen}
            title={chosen ? 'Klik untuk menghapus dari favorit' : blocked ? 'Maksimum 12 aplikasi' : 'Klik untuk menambahkan'}
            onclick={() => toggle(app)}
          >
            <span class="app-badge picker-badge">
              {#if island.appIcons[app.path]}
                <img class="picker-icon" src={island.appIcons[app.path]} alt="" draggable="false" />
              {:else}
                <span class="picker-initial">{app.name.charAt(0).toUpperCase()}</span>
              {/if}
            </span>
            <span class="truncate text-[11px] font-medium">{app.name}</span>
            {#if chosen}
              <span class="picker-mark picker-mark-on"><Icon name="check" class="size-2.5" /></span>
            {:else}
              <span class="picker-mark"><Icon name="plus" class="size-2.5" /></span>
            {/if}
          </button>
        </li>
      {/each}
      {#if results.length === 0}
        <li class="px-2 py-3 text-center text-[10px] text-[var(--muted)]">Tidak ada yang cocok</li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .head-btn {
    display: flex;
    height: 18px;
    align-items: center;
    gap: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--di-blue) 12%, transparent);
    padding: 0 8px;
    color: var(--di-blue);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-blue) 22%, transparent);
    font-size: 9px;
    font-weight: 500;
    transition: background 120ms ease;
  }

  .head-btn:hover {
    background: color-mix(in srgb, var(--di-blue) 20%, transparent);
  }

  .app-cell {
    position: relative;
    min-width: 0;
  }

  .app-tile {
    display: flex;
    width: 100%;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    border-radius: 12px;
    padding: 6px 2px;
    transition:
      background 120ms ease,
      transform 120ms var(--ease-out);
  }

  .app-tile:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }

  .app-tile:active {
    transform: scale(0.94);
  }

  .app-badge {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 11px;
    background: color-mix(in srgb, var(--tile-tone, var(--di-blue)) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tile-tone, var(--di-blue)) 26%, transparent);
    transition: box-shadow 120ms ease;
  }

  .app-tile:hover .app-badge {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--tile-tone, var(--di-blue)) 40%, transparent),
      0 2px 8px color-mix(in srgb, var(--tile-tone, var(--di-blue)) 18%, transparent);
  }

  .app-initial {
    font-size: 15px;
    font-weight: 700;
    line-height: 1;
    color: var(--tile-tone, var(--di-blue));
  }

  .app-icon {
    width: 30px;
    height: 30px;
    object-fit: contain;
  }

  .app-name {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 9px;
    color: var(--muted);
  }

  .app-remove {
    position: absolute;
    top: 1px;
    right: 2px;
    display: grid;
    place-items: center;
    width: 15px;
    height: 15px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--di-red) 90%, black);
    color: #fff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.35);
    opacity: 0;
    transform: scale(0.8);
    transition:
      opacity 120ms ease,
      transform 120ms var(--ease-out);
  }

  .app-cell:hover .app-remove {
    opacity: 1;
    transform: scale(1);
  }

  .app-add .app-badge {
    --tile-tone: var(--muted);
    background: color-mix(in srgb, var(--ink) 4%, transparent);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }

  .app-add:hover .app-badge {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 22%, transparent);
    color: var(--ink);
  }

  .app-empty {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: center;
  }

  .app-empty-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 13px;
    background: color-mix(in srgb, var(--di-blue) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-blue) 22%, transparent);
    color: var(--di-blue);
  }

  .app-empty-cta {
    display: flex;
    align-items: center;
    gap: 5px;
    border-radius: 999px;
    background: var(--di-blue);
    padding: 4px 12px;
    color: #fff;
    font-size: 10px;
    font-weight: 600;
    box-shadow: 0 3px 10px color-mix(in srgb, var(--di-blue) 30%, transparent);
    transition:
      transform 120ms var(--ease-out),
      filter 120ms ease;
  }

  .app-empty-cta:hover {
    filter: brightness(1.08);
  }

  .app-empty-cta:active {
    transform: scale(0.96);
  }

  .picker-search {
    min-width: 0;
    flex: 1;
    height: 24px;
    border-radius: 8px;
    border: none;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    padding: 0 10px;
    color: var(--ink);
    font-size: 11px;
    outline: none;
  }

  .picker-search:focus {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-blue) 45%, transparent);
  }

  .picker-count {
    flex-shrink: 0;
    font-size: 9px;
    color: var(--faint);
  }

  .picker-done {
    display: flex;
    height: 24px;
    flex-shrink: 0;
    align-items: center;
    gap: 4px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--di-green) 16%, transparent);
    padding: 0 9px;
    color: var(--di-green);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-green) 26%, transparent);
    font-size: 10px;
    font-weight: 600;
    transition: background 120ms ease;
  }

  .picker-done:hover {
    background: color-mix(in srgb, var(--di-green) 24%, transparent);
  }

  .picker-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--ink) 18%, transparent) transparent;
  }

  .picker-badge {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    --tile-tone: var(--di-blue);
  }

  .picker-initial {
    font-size: 11px;
    font-weight: 700;
    color: var(--di-blue);
  }

  .picker-icon {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }

  .picker-row {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 8px;
    border-radius: 8px;
    padding: 3px 6px;
    color: var(--muted);
    text-align: left;
    transition:
      background 100ms ease,
      color 100ms ease;
  }

  .picker-row > .truncate {
    flex: 1;
    min-width: 0;
  }

  .picker-row:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
    color: var(--ink);
  }

  .picker-row-on,
  .picker-row-on:hover {
    color: var(--di-green);
    background: color-mix(in srgb, var(--di-green) 12%, transparent);
  }

  .picker-row-blocked {
    opacity: 0.45;
    pointer-events: none;
  }

  .picker-mark {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border-radius: 6px;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
    color: var(--muted);
  }

  .picker-mark-on {
    background: color-mix(in srgb, var(--di-green) 22%, transparent);
    color: var(--di-green);
  }
</style>
