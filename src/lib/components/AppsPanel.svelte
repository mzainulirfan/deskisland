<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { getAppIcon, launchApp, listAppShortcuts, setFavorites } from '$lib/services/desktop';
  import type { Favorite } from '$lib/types/system';

  const MAX = 12;
  const TONES = ['var(--di-blue)', 'var(--di-purple)', 'var(--di-green)', 'var(--di-amber)', 'var(--di-red)'];
  // Icons are 48-px PNG reads that cost a GDI round-trip each, so only the rows that can
  // actually be seen are fetched: the favorite tiles plus the head of the launcher list.
  const VISIBLE_ROWS = 16;

  let query = $state('');
  let failure = $state('');
  let searchEl: HTMLInputElement | undefined;

  const favorites = $derived(island.favorites);
  const favoritePaths = $derived(new Set(favorites.map((item) => item.path)));
  const full = $derived(favorites.length >= MAX);
  const term = $derived(query.trim().toLowerCase());
  const results = $derived(
    term ? island.appShortcuts.filter((app) => app.name.toLowerCase().includes(term)) : island.appShortcuts,
  );
  const listed = $derived(results.slice(0, VISIBLE_ROWS));

  // The Start Menu scan is the expensive part, so it happens when this page is first shown
  // and never again: the store keeps the list for the rest of the session.
  let loading = false;
  $effect(() => {
    if (loading || island.appShortcuts.length) return;
    loading = true;
    void listAppShortcuts().then((list) => (island.appShortcuts = list));
  });

  const inFlight = new Set<string>();
  $effect(() => {
    const paths = [...favorites.map((item) => item.path), ...listed.map((app) => app.path)];
    for (const path of paths) {
      if (path in island.appIcons || inFlight.has(path)) continue;
      inFlight.add(path);
      void getAppIcon(path).then((url) => {
        inFlight.delete(path);
        if (url) island.appIcons[path] = url;
      });
    }
  });

  /** Favorites are optimistic: the grid changes now, the file confirms it. If the write is
   * rejected the previous list comes back so the grid is never ahead of settings.json. */
  function commit(next: Favorite[]) {
    const sorted = next
      .slice()
      .sort((a, b) => a.name.localeCompare(b.name, 'id', { sensitivity: 'base' }));
    const previous = island.favorites;
    island.favorites = sorted;
    setFavorites(sorted).then(
      () => (failure = ''),
      () => {
        island.favorites = previous;
        failure = 'Gagal menyimpan ke berkas setelan.';
      },
    );
  }

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

  // Right-click a tile to remove it without touching the launcher list.
  function onContext(event: MouseEvent, app: Favorite) {
    event.preventDefault();
    remove(app);
  }
</script>

<div class="apps-shell flex h-full min-h-0 flex-col gap-1.5 rounded-[15px] p-2.5 ring-1 ring-[color:var(--line)]">
  <div class="flex min-h-0 flex-1 gap-2.5">
    <!-- ============ LEFT: the tiles that launch with one click ============ -->
    <div class="flex min-w-0 flex-1 flex-col">
      <header class="flex h-6 shrink-0 items-center gap-1.5">
        <span class="size-1.5 rounded-full bg-[var(--di-blue)]" aria-hidden="true"></span>
        <h2 class="text-[9px] font-medium uppercase tracking-[0.08em] text-[var(--muted)]">
          Aplikasi favorit
        </h2>
        <span class="text-[9px] text-[var(--faint)]">{favorites.length} / {MAX}</span>
        {#if favorites.length}
          <button
            type="button"
            class="apps-clear-all"
            aria-label="Hapus semua aplikasi favorit"
            title="Hapus semua"
            onclick={() => commit([])}
          >
            <Icon name="trash" class="size-2.5" />
          </button>
        {/if}
      </header>

      {#if favorites.length}
        <ul class="apps-grid mt-0.5 min-h-0 flex-1" aria-label="Aplikasi favorit">
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
                aria-label="Pilih aplikasi dari daftar di kanan"
                title="Pilih dari daftar Start Menu di kanan"
                onclick={() => searchEl?.focus()}
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
            Belum ada aplikasi favorit.<br />Pilih dari daftar Start Menu di kanan.
          </p>
        </div>
      {/if}
      {#if failure}
        <p class="shrink-0 pt-1 text-[9px] font-semibold text-[var(--di-red)]">{failure}</p>
      {/if}
    </div>

    <!-- ============ RIGHT: the Start Menu launcher ============ -->
    <div class="apps-launch flex w-[214px] shrink-0 flex-col">
      <div class="relative h-6 shrink-0">
        <Icon
          name="search"
          class="pointer-events-none absolute top-1/2 left-2.5 size-3 -translate-y-1/2 text-[var(--faint)]"
        />
        <input
          class="apps-search"
          type="text"
          placeholder="Cari pintasan…"
          aria-label="Cari pintasan Start Menu"
          bind:value={query}
          bind:this={searchEl}
        />
        {#if query}
          <button
            type="button"
            class="apps-clear"
            aria-label="Bersihkan pencarian"
            onclick={() => (query = '')}
          >
            <Icon name="close" class="size-2.5" />
          </button>
        {/if}
      </div>

      <p class="apps-hint shrink-0">
        {term
          ? `${results.length} cocok`
          : `${island.appShortcuts.length} pintasan Start Menu`}
        · klik untuk membuka
      </p>

      <ul class="apps-rows min-h-0 flex-1" aria-label="Daftar pintasan">
        {#each listed as app (app.path)}
          {@const chosen = favoritePaths.has(app.path)}
          {@const blocked = !chosen && full}
          <li>
            <div class="launch-row" class:launch-row-on={chosen}>
              <button
                type="button"
                class="launch-main"
                title="Buka {app.name}"
                onclick={() => void launchApp(app.path)}
              >
                <span class="app-badge launch-badge" class:launch-badge-on={chosen}>
                  {#if island.appIcons[app.path]}
                    <img class="launch-icon" src={island.appIcons[app.path]} alt="" draggable="false" />
                  {:else}
                    <span class="launch-initial">{app.name.charAt(0).toUpperCase()}</span>
                  {/if}
                </span>
                <span class="launch-name">{app.name}</span>
              </button>
              <button
                type="button"
                class="launch-mark"
                class:launch-mark-on={chosen}
                class:launch-mark-blocked={blocked}
                disabled={blocked}
                aria-pressed={chosen}
                aria-label={chosen ? `Hapus ${app.name} dari favorit` : `Tambah ${app.name} ke favorit`}
                title={chosen
                  ? 'Hapus dari favorit'
                  : blocked
                    ? `Maksimal ${MAX} aplikasi favorit`
                    : 'Tambah ke favorit'}
                onclick={() => toggle(app)}
              >
                <Icon name={chosen ? 'check' : 'plus'} class="size-2.5" />
              </button>
            </div>
          </li>
        {/each}
        {#if results.length === 0}
          <li class="apps-none">Tidak ada pintasan yang cocok.</li>
        {:else if results.length > VISIBLE_ROWS}
          <li class="apps-none">
            {results.length - VISIBLE_ROWS} lagi — persempit dengan pencarian.
          </li>
        {/if}
      </ul>
    </div>
  </div>
</div>

<style>
  .apps-shell {
    --tint: var(--di-blue);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--tint) 7%, transparent), transparent 55%),
      var(--card);
  }

  .apps-launch {
    padding-left: 10px;
    border-left: 1px solid var(--line);
  }

  /* ---- favorites grid ---- */
  .apps-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    /* `safe` matters: a plain `center` would push the first row above the scroll area once
     * the list is full, and that row could never be reached again. */
    align-content: safe center;
    gap: 2px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--ink) 18%, transparent) transparent;
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
    gap: 3px;
    border-radius: 11px;
    padding: 5px 2px;
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
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: color-mix(in srgb, var(--tile-tone, var(--di-blue)) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tile-tone, var(--di-blue)) 24%, transparent);
    transition: box-shadow 120ms ease;
  }

  .app-tile:hover .app-badge {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--tile-tone, var(--di-blue)) 42%, transparent),
      0 2px 9px color-mix(in srgb, var(--tile-tone, var(--di-blue)) 20%, transparent);
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
    top: 2px;
    right: 4px;
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

  .apps-clear-all {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-left: auto;
    border-radius: 6px;
    color: var(--faint);
    transition:
      background 120ms ease,
      color 120ms ease;
  }

  .apps-clear-all:hover {
    background: color-mix(in srgb, var(--di-red) 14%, transparent);
    color: var(--di-red);
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

  /* ---- launcher ---- */
  .apps-search {
    width: 100%;
    height: 24px;
    border: none;
    border-radius: 8px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    padding: 0 24px;
    color: var(--ink);
    font-size: 10.5px;
    outline: none;
    transition: box-shadow 120ms ease;
  }

  .apps-search::placeholder {
    color: var(--faint);
  }

  .apps-search:focus {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-blue) 50%, transparent),
      0 0 0 3px color-mix(in srgb, var(--di-blue) 10%, transparent);
  }

  .apps-clear {
    position: absolute;
    top: 50%;
    right: 5px;
    display: grid;
    place-items: center;
    width: 15px;
    height: 15px;
    translate: 0 -50%;
    border-radius: 999px;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
    color: var(--muted);
  }

  .apps-clear:hover {
    color: var(--ink);
  }

  .apps-hint {
    padding: 4px 2px 3px;
    font-size: 8.5px;
    color: var(--faint);
  }

  .apps-rows {
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--ink) 18%, transparent) transparent;
    /* The list ends mid-row most of the time; the fade says "there is more below" instead of
     * looking like the panel was cut off. */
    mask-image: linear-gradient(180deg, black calc(100% - 22px), transparent);
  }

  .launch-row {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: 8px;
    padding-right: 3px;
    transition: background 100ms ease;
  }

  .launch-row:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }

  .launch-row-on,
  .launch-row-on:hover {
    background: color-mix(in srgb, var(--di-green) 10%, transparent);
  }

  .launch-main {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: 7px;
    padding: 2px 0 2px 4px;
    text-align: left;
  }

  .launch-badge {
    --tile-tone: var(--di-blue);
    width: 22px;
    height: 22px;
    border-radius: 7px;
    flex-shrink: 0;
  }

  .launch-badge-on {
    --tile-tone: var(--di-green);
  }

  .launch-initial {
    font-size: 11px;
    font-weight: 700;
    color: var(--tile-tone, var(--di-blue));
  }

  .launch-icon {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }

  .launch-name {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 10.5px;
    color: var(--muted);
  }

  .launch-row:hover .launch-name {
    color: var(--ink);
  }

  .launch-mark {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border-radius: 6px;
    background: color-mix(in srgb, var(--ink) 7%, transparent);
    color: var(--muted);
    transition:
      background 100ms ease,
      color 100ms ease;
  }

  .launch-mark:hover {
    background: color-mix(in srgb, var(--di-blue) 22%, transparent);
    color: var(--di-blue);
  }

  .launch-mark-on {
    background: color-mix(in srgb, var(--di-green) 20%, transparent);
    color: var(--di-green);
  }

  .launch-mark-on:hover {
    background: color-mix(in srgb, var(--di-red) 18%, transparent);
    color: var(--di-red);
  }

  .launch-mark-blocked {
    opacity: 0.35;
  }

  .apps-none {
    padding: 6px 4px;
    font-size: 9.5px;
    color: var(--faint);
  }
</style>
