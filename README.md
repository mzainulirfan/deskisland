# Desk Island

Dynamic Island untuk Windows 10/11: pill kecil di tepi atas layar yang melebar saat ada aktivitas, dan membuka panel berisi pemantauan sistem, media, timer, pengingat harian, proses, unduhan, aplikasi, dan pintasan setelan Windows.

Stack: Svelte 5 (runes) + Tauri 2 + Rust + Tailwind CSS 4. Hanya Windows; tidak ada jalur kode lintas platform yang benar-benar dipakai.

## Menjalankan

```powershell
npm install
npm run tauri dev      # aplikasi Tauri (butuh WebView2)
```

- `npm run dev` saja menjalankan pill di browser: semua command dibungkus `inTauri()` sehingga aplikasi tetap jalan penuh dengan nilai default, hanya tanpa data OS. `?poll=1` memaksa mode polling browser meskipun di dalam Tauri.
- `npm run check` = `svelte-check` (hanya `src/`; `src-tauri` di-exclude dari tsconfig, jadi `cargo check` tetap harus terpisah).
- `npm run tauri build` → `src-tauri/target/release/desk-island.exe` dan installer NSIS per-user di `src-tauri/target/release/bundle/nsis/Desk Island_0.1.0_x64-setup.exe`.

## Status

### Island

- Pill transparan, borderless, always-on-top, `skipTaskbar`, `focus: false`. Jendela native selalu `EXPANDED_SIZE + 2×PAD` (672+64 × 358+64 = 736×422) dan punya `PAD` 32 px ruang kosong di sekelilingnya supaya glow tidak terpotong; margin layar 16 px logis, jarak dari tepi atas bisa diatur 0–200 px lewat setelan.
- Ukuran yang benar: idle per varian (min 148, timer/battery 156, mon 260, music 264, eyes 282, notification 320 — semuanya tinggi 50), notice 320×50, welcome 288×50, HUD minimal 240 px, panel maksimum 672×358.
- Tujuh varian idle (eyes, min, music, mon, timer, battery, notification) plus `auto` yang berganti tiap 6 detik. Varian `eyes`/`min` memakai maskot yang pupilnya melacak posisi kursor OS; skala maskot bisa diatur.
- Mesin mode `idle → hud → notice → panel`. `hud` (volume, tombol) dan `notice` (unduhan, notifikasi Windows, timer, pengingat) punya deadline sendiri (`hudUntil`, `noticeUntil`); HUD menang saat aktif, notice muncul lagi setelahnya. Notice timer yang sedang berbunyi tidak bisa kedaluwarsa sampai di-dismiss.
- Panel punya delapan tab: **Sistem** (kartu CPU/RAM/Jaringan/Disk + sparkline), **Media** (sesi GSMTC + kontrol playback, volume master digabung di halaman yang sama), **Timer & Pomodoro**, **Pengingat**, **Proses**, **Unduhan**, **Aplikasi** (kolom kiri: kisi 4 kolom favorit yang di-launch dengan sekali klik; kolom kanan: peluncur Start Menu dengan pencarian inline — baris membuka aplikasi, tanda di ujung baris menambah/menghapus favorit), dan **Cepat** (enam pintasan halaman Setelan Windows: Wi-Fi, Bluetooth, Mode malam, Fokus, Mikrofon, Kecerahan).
- Navigasi panel ada dua layout dan keduanya nyata: `rail` (menu vertikal di kiri, satu pane; jendela tetap 358 px karena rail adalah elemen tertingginya, tapi pane hanya setinggi tab-nya — `PANEL_HEIGHTS[tab] - 72` — dan rata atas, tidak melar mengikuti rail) dan `tabs` (strip horizontal di header + track geser; tiap tab punya tinggi sendiri di `PANEL_HEIGHTS`, 176–296, clamp 150–358). `PANEL_HEIGHTS` dan `PANEL_MIN_HEIGHT` tinggal di `src/lib/panel-metrics.ts` supaya kedua layout memakai angka yang sama; 72 px-nya adalah chrome yang identik di keduanya (header 48 + celah 12 + `pb-3` 12). Di layout rail, `h-full` pada pane juga yang membuat scrollbar vertikal berkedip tiap ganti halaman: animasi `pane-in` menggeser isi 4 px ke bawah di dalam kolom `overflow-y-auto`, dan karena pane persama tinggi dengan kolomnya, 4 px itu jadi overflow. Di kedua layout, halaman terbuka **cukup dengan men-hover icon tab**-nya (jeda intent 120 ms supaya menyapu menu tidak membuka semua halaman); klik tetap berfungsi dan langsung memindahkan tanpa menunggu. Jendela *settle* 350 ms sesudah perpindahan dibutuhkan: di layout `tabs` tab yang aktif melebar karena labelnya muncul, jadi icon berikutnya bergeser tepat ke bawah kursor yang diam dan Chromium mengirim `pointerenter` baru untuk element yang berpindah itu — tanpa jeda, halaman "berjalan" sendiri menjauhi kursor.
- Overlay sambutan (welcome) saat island baru muncul atau kembali dari tersembunyi: teks diketik per karakter, durasi dan isinya bisa diatur, termasuk "kolam" sambutan yang dirotasi tiap kali island muncul kembali.
- Keluar hanya lewat tray. Klik kiri icon = sembunyikan/tampilkan, klik kanan = menu (Sembunyikan island / Buka panel / Mulai otomatis / Setelan / Keluar). Hotkey global `Ctrl+Alt+Space` membuka panel dan **mem-pin**-nya; panel yang di-pin tidak collapse saat kursor keluar. Saat aplikasi fullscreen aktif, pill disembunyikan dan hotkey hanya menampilkannya sementara ("peek").

### Deteksi hover dan klik-tembus

Karena jendela click-through saat pill menutup, `pointerenter`/`pointerleave` DOM tidak bisa dipakai. Sebuah thread di Rust mem-`GetCursorPos` tiap 12 ms dan membandingkannya dengan rect region yang sedang digambar (region panel diberi kelonggaran 4 px). Masuk dihitung setelah 2 sampel (~24 ms), keluar setelah 3 sampel (~36 ms), jadi kursor yang cuma lewat tidak memicu morph. Selain edge, thread itu mengirim posisi kursor ternormalisasi maksimal 30 fps khusus untuk melacak pupil mata, dan berhenti mengirim sama sekali saat lebar pill > 400 px (panel terbuka, mata tidak digambar). `set_island_passthrough` (`WS_EX_TRANSPARENT`) aktif setiap kali kondisi "panel terbuka ATAU kursor di atas pill" tidak terpenuhi. Setelah 30 detik tanpa interaksi pill meredup, dan kembali terang saat kursor mendekat.

### Data dan pemicu latar

Semua pemicu hidup di thread Rust, bukan interval frontend, supaya tetap berbunyi saat island tersembunyi atau webview sedang tidur:

- **Sistem** — CPU, RAM, throughput jaringan (rx/tx per detik), baterai, dan pemakaian disk (`sysinfo` + `GetSystemPowerStatus`) dibaca frontend tiap `poll_ms` (default 1000 ms) lewat `system_stats`. Volume dipoll 500 ms, media 3000 ms; data berat (proses, unduhan, media) hanya diambil saat tab-nya tampil.
- **Timer & Pomodoro** — hitung mundur dimiliki backend (`TimerInfo` dengan deadline), thread-nya tidur adaptif 200 ms saat jalan / 1000 ms saat idle / 3000 ms saat alarm. Alarm mengulang `MessageBeep` maksimal 20 kali sampai di-dismiss. Siklus Pomodoro (fase, ronde, durasi tiap fase, streak hari ini) dimiliki `island.pomo` di store dan dicerminkan ke `localStorage`, jadi posisinya tidak hilang saat panel timer belum pernah dibuka; pindah fase dikerjakan `island.nextPomoPhase(completed)` dan ronde yang di-skip tidak menambah streak.
- **Pengingat harian** — sampai 12 alarm `"HH:MM"` dengan pilihan hari (bitmask Senin–Minggu) di `reminders.json`. Backend punya datanya (`get_reminders` / `save_reminders` / `preview_reminder`); satu thread memeriksa jam dinding tiap 2 detik, dan saat menit plus harinya cocok island dipaksa muncul, pill morph jadi notice, dan `MessageBeep` berbunyi (frontend menambah chime WebAudio tiga nada). Berkas yang diedit manual ikut diperbaiki saat dimuat (jam tak valid dibulatkan, teks dipotong, id diisi) dan satu alarm hanya berbunyi sekali per hari karena menit yang sudah dipakai dicatat sebagai `tanggal#id`.
- **Notifikasi Windows** — dibaca read-only dari `wpndatabase.db` (SQLite) tiap 500 ms; `UserNotificationListener` tidak bisa dipakai aplikasi desktop tanpa identitas paket. Pembacaan pertama hanya mencatat posisi terakhir supaya toast lama tidak diputar ulang, dan seluruh notifikasi diabaikan saat `notifications: false`. Kalau beberapa toast muncul dalam satu window polling, semuanya diterbitkan (terlama lebih dulu) supaya pill berhenti di notifikasi terbaru dan yang menyusul tidak hilang.
- **Unduhan** — folder Downloads di-poll (`read_dir` + jeda settle 2 detik), Zone.Identifier dihapus supaya hasil unduhan bisa langsung dibuka.
- **Keyboard** — low-level hook `WH_KEYBOARD_LL` pada thread dengan message loop sendiri, hanya membaca whitelist (Tab, Enter, Win, Esc, Space, Backspace, panah, Caps Lock); teks ketikan tidak pernah disentuh. Esc dipakai untuk menutup panel karena webview biasanya tidak punya fokus keyboard.
- **Fullscreen** — `SHQueryUserNotificationState` dicek tiap 1500 ms dan event hanya dikirim saat nilainya berubah.

### Setelan

Satu jendela `settings` terpisah, dibuka dari gear di header panel atau menu tray, berisi empat halaman: **Tampilan**, **Sambutan**, **Perilaku**, **Integrasi**. Semuanya disimpan ke `settings.json` lewat backend dan dipulihkan saat start; nilai di luar rentang dibetulkan saat baca dan saat tulis. Tiga jalur tulis (`set_settings`, `set_favorites`, `save_reminders`) menulis dulu baru mengirim event, dan error-nya diteruskan ke UI: pill "Gagal menyimpan" di jendela Setelan, baris merah di panel Pengingat, dan daftar favorit yang kembali ke state sebelumnya. Status autostart disinkronkan dua arah antara jendela setelan dan checkbox tray lewat event `autostart-changed`.

## Struktur

```
src/main.ts                 bootstrap Svelte; memilih view lewat window.__DESK_VIEW__
src/App.svelte              mount island, polling, dengar semua event backend, chime WebAudio
src/Settings.svelte         jendela Setelan (4 halaman)
src/app.css                 token --di-*, alias semantik, tema gelap/terang, motion, reduced-motion
src/lib/stores/island.svelte.ts   satu-satunya state: mode, overlay+deadline, panelTab, nav,
                                  pinned, fullscreen, stats, volume, media, processes, downloads,
                                  favorites, appShortcuts, reminders, timer, posisi, tema, timing
src/lib/services/desktop.ts  pembungkus invoke() dengan guard inTauri() dan nilai fallback
src/lib/types/system.ts     tipe payload yang sama dengan sisi Rust
src/lib/greetings.ts        parser kolam sambutan
src/lib/components/         Island (morph+hover+pin+resize+welcome), SystemPanel (shell+nav),
                            IdleView, HudView, NoticeView, WelcomeView, Mascot, Eyes,
                            MediaPanel, VolumePanel, TimerPanel, RemindersPanel, ProcessPanel,
                            DownloadsPanel, AppsPanel, QuickSettingsPanel,
                            MetricCard, StatTile, Sparkline, SliderRow, Icon
src-tauri/src/main.rs       flag WebView2 (anti-backgrounding) + run()
src-tauri/src/lib.rs        builder, plugin, managed state, setup, generate_handler
src-tauri/src/modules/system.rs       sampling CPU/RAM/jaringan/baterai/disk (Monitor persisten)
src-tauri/src/modules/process.rs      proses terpanas (nama sama satu exe digabung, CPU <= 0,5 dibuang)
src-tauri/src/modules/audio.rs        volume master + mute (Core Audio)
src-tauri/src/modules/media.rs        sesi GSMTC + kontrol playback
src-tauri/src/modules/downloads.rs    watcher folder Downloads
src-tauri/src/modules/notifications.rs pembaca read-only database notifikasi WPN
src-tauri/src/modules/timer.rs        countdown backend, thread alarm, MessageBeep
src-tauri/src/modules/reminders.rs    jadwal pengingat harian, watcher jam dinding, CRUD
src-tauri/src/modules/apps.rs         walk Start Menu, icon -> base64 PNG, favorit
src-tauri/src/modules/quick.rs        pembuka halaman Setelan Windows + Task Manager
src-tauri/src/modules/settings.rs     settings.json, sanitasi, jendela Setelan
src-tauri/src/modules/autostart.rs    status/toggle autostart + sinkronisasi tray
src-tauri/src/modules/tray.rs         icon tray, menu, keluar
src-tauri/src/modules/keyboard.rs     hook WH_KEYBOARD_LL untuk HUD tombol
src-tauri/src/modules/fullscreen.rs   SHQueryUserNotificationState + fullscreen-changed
src-tauri/src/modules/window.rs       fit/place/anchor, region hover, passthrough, monitor kursor
src-tauri/src/modules/com.rs          CoInitializeEx per thread + helper blocking WinRT
src-tauri/examples/media_probe.rs     perkakas bantu saat mengembangkan modul media
```

Berkas data di `%APPDATA%\com.desk.island`: `settings.json` (snake_case) dan `reminders.json` (camelCase). Keduanya jatuh ke `temp_dir()` kalau app data dir tidak tersedia.

## Catatan desain

**Jendela tidak pernah di-resize saat animasi, tapi memang berubah ukuran antar tab.** `fit_island_window` memanggil `set_size` + `place`, dan frontend memanggilnya tiap pindah tab dengan strategi "naik langsung, turun ditunda 220 ms": saat mengecil jendela menahan tinggi lama dulu supaya sudut glow yang membulat tidak terpotong tepi jendela di tengah transisi. Catatan komentar di `window.rs:152` ("never resized afterwards") sudah tidak benar — yang benar adalah tidak ada resize *selagi* morph berjalan.

**Glow dibuat inset**, bukan luar, karena jendela Tauri transparan berukuran sama dengan island: pada panel penuh, glow luar akan terpotong batas jendela dan tampak kotak.

**Posisi (kiri/tengah/kanan) menggeser footprint, bukan pill.** Menengah memakai lebar jendela, kiri/kanan memakai lebar island, keduanya dikurangi `PAD`, supaya panel yang melebar tidak terpotong keluar layar.

**Geometri window di-cache** (`GEOM`, `REGION` di `window.rs`) karena empat panggilan window di thread utama tiap 40 ms dulu membuat rendering, emit, dan pelacakan mata kelaparan.

**Panel memakai tab state, bukan rel track permanen.** Di layout `tabs` yang mounted hanya tiga pane (sebelum/sesudah) demi animasi geser; di layout `rail` satu pane saja dan isinya di-align ke atas, bukan diregangkan, supaya tinggi tetap 358 tidak menyisakan ruang mati.

**Sparkline menyimpan histori di komponen**, dan nilai jaringan memakai skala dinamis dari sampel tertinggi supaya tetap terbaca saat lalu lintas kecil.

**Tema memakai token** `--di-*` di `app.css` plus alias lama (`--surface`, `--ink`, `--muted`, `--accent`, dst.) dan atribut `data-theme` di `<html>`; komponen tidak punya warna sendiri kecuali beberapa permukaan maskot dan tombol accent yang memang sengaja tetap gelap di kedua tema. Hue glow per tab diambil dari `--glow-rgb` lewat selektor `[data-tab=...]`. Nilai tersimpan dibaca sebelum render pertama agar tema terang tidak berkedip gelap.

**Motion** memakai `--ease-island`/`--ease-pop` tanpa overshoot dengan durasi `morph_ms` (default 420), dan seluruh animasi besar menghormati `prefers-reduced-motion`. Kedipan mata sengaja CSS-nya tidak pakai animasi tak berujung supaya kompositor WebView2 boleh tidur.

**COM diinisialisasi per thread** (`com.rs`) karena Tauri menjalankan command di thread pool-nya; kalau thread itu sudah STA, `CoInitializeEx` menolak mode MTA (`RPC_E_CHANGED_MODE`) dan itu dianggap sukses. WinRT async (GSMTC) di-block dengan poll-loop berduri 2 detik supaya pemutar yang macet tidak memarkir thread command selamanya.

**Pengingat memakai jam dinding lokal** (`GetLocalTime`, bukan `SystemTime` UTC) supaya alarm mengikuti hari dan zona waktu pengguna; chrono sengaja tidak ditambahkan hanya untuk membaca tanggal. `wDayOfWeek` Windows (0 = Minggu) dikonversi ke bitmask Senin-first lewat `1 << ((wDayOfWeek + 6) % 7)` agar satu bit = satu chip hari di UI. CRUD-nya satu kali tulis daftar penuh (`save_reminders` menerima seluruh daftar, backend menormalkan lalu mengembalikan hasilnya) mengikuti pola `set_favorites`, jadi frontend tidak pernah punya id karangan.

**Payload notifikasi tidak pernah disalin atau disimpan** — event hanya membawa judul, isi, dan nama app. Kalau database dibuat ulang sehingga `Order` turun, watcher melakukan rebase tanpa memutar notifikasi lama.

**Tray memakai managed state** untuk dua item yang berubah saat runtime (label tampil/sembunyi dan checkbox autostart). Autostart memakai plugin resmi dengan entri HKCU Run per-user; saat dev entri itu menunjuk `target\debug\desk-island.exe`, jadi setelah install ulang toggle-nya sekali supaya menunjuk executable hasil instalasi.

**Jendela Setelan dibuat dari command async** (`open_settings`) untuk menghindari deadlock WebView2 saat membuat window dari command sinkron di Windows. View-nya dipilih lewat initialization script (`window.__DESK_VIEW__ = 'settings'`), bukan query string.

**`main.rs` men-set flag WebView2** (`--disable-backgrounding-occluded-windows` dan kerabatnya); tanpa itu jendela transparan always-on-top dianggap "occluded" dan renderer-nya dibatasi sampai animasi patah-patah.

## Roadmap berikutnya

- Code signing dan uji installer di mesin bersih.
- `panic = "abort"` + `.expect("... poisoned")` di delapan tempat berarti satu panic di thread mana pun mematikan seluruh aplikasi; perlu keputusan sadar (memperkecil `.expect` atau membatasi dampak panic).
- Lint/format dan CI belum ada sama sekali (`npm run check`, `cargo check`, dan build tidak dijaga otomatis).
- Beberapa warna permukaan maskot dan tombol accent tidak ikut tema terang.
- `set_island_position` mengubah margin atas di sisi Rust tanpa menulis `settings.json`; ia hanya awet kalau frontend juga memanggil `set_settings`.
- `desktop.ts` masih mengekspor `playSystemBeep` yang tidak pernah dipanggil, dan `quick.rs` menyokong bagian display/battery/screenshot yang tidak ditampilkan di UI.
