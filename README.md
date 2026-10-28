# Desk Island

Dynamic Island untuk Windows 10/11: pill hitam di tengah atas layar yang melebar saat ada aktivitas.

Stack: Svelte 5 + Tauri 2 + Rust + Tailwind CSS 4.

## Menjalankan

```powershell
npm install
npm run tauri dev
```

Build installer NSIS: `npm run tauri build` â†’ `src-tauri/target/release/bundle/nsis/Desk Island_0.1.0_x64-setup.exe`.

## Status: Tahap 4 (desain ulang, tray, autostart, setelan & installer)

- Pill transparan, borderless, always-on-top, menempel di atas monitor utama (margin 14 logical px) dengan tiga anchor: kiri, tengah, kanan.
- Tiga ukuran: idle 320×48, notice 460×72, panel 840×248. Jendela native difit ke ukuran panel saat start lalu tidak di-resize; area di luar pill/panel dibuat click-through.
- Idle menampilkan mark, jam, divider, indikator aktivitas media, ikon jaringan, dan chip baterai. Hover punya jeda intent Â±120 ms sebelum panel terbuka, dan pill meredup setelah 30 detik tanpa interaksi.
- Panel memakai delapan tab: **Sistem**, **Media**, **Timer & Pomodoro**, **Pengingat**, **Proses**, **Unduhan**, **Aplikasi**, dan **Cepat**. Sistem berisi kartu CPU/RAM/Jaringan/Disk dengan sparkline; Media menampilkan sesi GSMTC dan volume; Timer & Pomodoro mengelola hitung mundur standar serta siklus fokus Pomodoro (fokus, rehat, rehat panjang, dan pelacak produktivitas); Proses menampilkan 6 proses teratas; Unduhan memantau berkas unduhan; Aplikasi menampilkan daftar aplikasi favorit & peluncur cepat; Cepat membuka halaman Setelan Windows dan Task Manager.
- Pengingat harian berulang: maksimal 12 alarm "HH:MM" dengan pilihan hari (bitmask Senin-Minggu). Backend yang punya datanya (`get_reminders` / `save_reminders` / `preview_reminder`, tersimpan di `reminders.json` di app data dir) dan sebuah thread watcher memeriksa jam dinding tiap 2 detik. Saat menit dan harinya cocok, island muncul walau sedang disembunyikan, pill morph jadi notice "teks / Pengingat HH:MM", dan `system_beep` berbunyi; klik notice membuka tab Pengingat. Berkas yang diedit manual ikut diperbaiki (jam tak valid dibulatkan ke menit, teks dipotong, id diisi), dan satu alarm hanya berbunyi sekali per hari karena menit yang sudah dipakai dicatat sebagai `tanggal#id` di memori.
- Folder Downloads dipantau dari Rust (`downloads_status`, polling `read_dir` + jeda 2 detik sebelum file dianggap selesai ditulis). File baru membuat pill morph ke notifikasi 460×72 ("nama berkas Â· terunduh Â· ukuran") selama 6 detik; klik di notifikasi langsung membuka panel.
- Notifikasi Windows (Action Center) dicerminkan ke pill: icon lonceng, judul, lalu "nama app Â· isi" selama 5,5 detik. Toast baru dibaca dari database WPN (`wpndatabase.db`) secara read-only tiap 500 ms â€” `UserNotificationListener` tidak bisa dipakai aplikasi desktop tanpa identitas paket. Pembacaan pertama hanya mencatat posisi terakhir supaya notifikasi lama tidak diputar ulang, dan notifikasi diabaikan saat panel sedang terbuka. Fitur ini ikut mati kalau notifikasi Windows dimatikan (`ToastEnabled=0`), karena Windows tidak menulis toast ke database.
- Posisi, tema, autostart, notifikasi, dan timing diatur dari jendela Setelan yang dibuka lewat gear di header panel atau menu tray. Nilainya disimpan di `settings.json` lewat backend dan dipulihkan saat aplikasi dijalankan ulang; tema diterapkan lewat CSS variable (`data-theme` di `<html>`). Metrik idle sudah dihapus dari model setelan.
- Saat idle, perubahan volume (media key, mixer, aplikasi lain) memunculkan HUD volume Â±1,2â€“1,4 detik di pill: icon speaker, bar, persen, dan "Bisu" saat mute. Volume dipoll tiap 300 ms hanya selama panel tertutup; saat panel terbuka, panel sendiri yang jadi sumber data.
- Tombol Tab, Enter, Win (juga Esc, Space, Backspace, panah, Caps Lock) memunculkan HUD berisi icon tombol + namanya dengan keterangan "ditekan" â€” tanpa label "Keyboard". Dideteksi lewat low-level hook di `modules/keyboard.rs` yang hanya membaca whitelist tombol itu â€” teks yang diketik tidak pernah disentuh, dan keyup tidak ditampilkan.
- CPU, RAM, throughput jaringan (rx/tx per detik), baterai, dan pemakaian disk dibaca dari Rust (`sysinfo` + `GetSystemPowerStatus`) lewat `system_stats`, dipolling 1 detik sekali.
- Volume master (Core Audio `IAudioEndpointVolume`): bar di panel jadi slider â€” klik atau geser untuk mengatur level; tombol speaker di kanan melakukan mute/unmute (`set_master_mute`). Per-app volume belum didukung dan ditandai jelas di UI.
- Daftar 6 proses terpanas menurut CPU lewat `top_processes` (`sysinfo`, `ProcessesToUpdate::All`); nama proses ditampilkan lengkap dan dipotong oleh UI bila terlalu panjang.
- Sesi media (GSMTC `GlobalSystemMediaTransportControlsSessionManager`) lewat `now_playing`: judul, artis, app, posisi/durasi. Panel menampilkan `â€”` dan "tidak ada pemutar" selama tidak ada aplikasi yang menerbitkan sesi media. Tombol playback belum diimplementasikan dan tidak ditampilkan sebagai kontrol palsu.
- Data volume/media/proses diambil saat panel terbuka karena masing-masing butuh round-trip COM. Saat panel tertutup, volume tetap dipoll tiap 300 ms dan media dipoll lambat tiap 3 detik khusus untuk status idle.
- Keluar dari aplikasi lewat menu tray. Icon tray tetap ada saat island disembunyikan; klik kiri icon untuk menyembunyikan/menampilkan pill, klik kanan membuka menu Sembunyikan island / Buka panel (Ctrl+Alt+Space) / Mulai otomatis / Setelan / Keluar. Hotkey global `Ctrl+Alt+Space` membuka panel dan mem-pin-nya; tekan lagi untuk menutup. Panel yang di-pin tidak collapse saat kursor keluar. Saat aplikasi fullscreen/game aktif, pill otomatis disembunyikan; hotkey tetap bisa menampilkan panel sementara. Di Windows 11 icon baru mendarat di overflow â€” buka lewat chevron atau pin ke taskbar supaya menu kanan-klik selalu terlihat.
- Jendela Setelan berisi posisi island, tema, toggle "Mulai otomatis" (menulis/menghapus entri HKCU Run lewat `tauri-plugin-autostart`), toggle "Notifikasi Windows" (preferensi lokal untuk menahan toast dari pill), dan timing HUD/notice/poll. Status autostart disinkronkan dua arah antara jendela setelan dan checkbox menu tray lewat event `autostart-changed`.
- Jendela Setelan dibuat lewat command async (`open_settings`) untuk menghindari deadlock WebView2 saat membuat window dari command sinkron di Windows. View-nya dipilih lewat initialization script (`window.__DESK_VIEW__ = 'settings'`), bukan query string.
- Installer Windows memakai NSIS mode `currentUser` (`npm run tauri build`), dengan icon aplikasi yang sama untuk executable dan installer.

## Struktur

```
src/lib/components/   Island.svelte (morph + hover + pin + fullscreen), IdleView, NoticeView, SystemPanel,
                      MetricCard, Sparkline, MediaPanel, VolumePanel, TimerPanel, RemindersPanel, ProcessPanel,
                      QuickSettingsPanel, Icon
src/lib/stores/       island.svelte.ts — mode, overlay, panelTab, pinned, fullscreen, time, stats, volume,
                      media, processes, notice, posisi, tema, timer, reminders
src/lib/services/     desktop.ts — pembungkus invoke()
src/app.css           token desain --di-* + alias lama, tema gelap/terang, motion, reduced-motion
src-tauri/src/modules/system.rs     sampling CPU, RAM, jaringan, baterai, disk (Monitor persisten di AppState)
src-tauri/src/modules/process.rs    daftar proses terpanas
src-tauri/src/modules/audio.rs      volume master Core Audio + mute
src-tauri/src/modules/media.rs      sesi media GSMTC
src-tauri/src/modules/downloads.rs  watcher folder Downloads (polling + jeda settle)
src-tauri/src/modules/notifications.rs  pembaca read-only database notifikasi WPN (polling 500 ms)
src-tauri/src/modules/timer.rs      countdown backend, watcher deadline thread, sistem alarm & audio repeat
src-tauri/src/modules/reminders.rs  jadwal pengingat harian (reminders.json), watcher jam dinding, command CRUD
src-tauri/src/modules/quick.rs      command halaman Setelan Windows + Task Manager
src-tauri/src/modules/settings.rs   settings.json, command setelan, open_settings async
src-tauri/src/modules/tray.rs       icon tray, menu tampil/sembunyi, autostart dan keluar
src-tauri/src/modules/autostart.rs  command status/toggle autostart + sinkronisasi ke tray
src-tauri/src/modules/keyboard.rs   low-level keyboard hook (whitelist Tab/Enter/Win dan beberapa tombol lain) untuk HUD tombol
src-tauri/src/modules/fullscreen.rs deteksi aplikasi fullscreen via SHQueryUserNotificationState + event fullscreen-changed
src-tauri/src/modules/com.rs        CoInitializeEx per thread + blocking helper untuk WinRT async
src-tauri/src/modules/window.rs     fit_island_window, set_island_position, cursor_over_island, set_island_passthrough, quit_app
```

## Catatan desain

Jendela native difit ke ukuran panel maksimum (840×248, `tauri.conf.json`) saat start dan tidak di-resize lagi; posisinya hanya berubah saat anchor diganti. Saat idle/notice, area di luar pill/notice dibuat click-through dengan `set_island_passthrough` (`WS_EX_TRANSPARENT`), jadi klik tembus ke aplikasi di belakangnya. `fit_island_window` dan `set_island_position` memakai satu helper `place()` supaya margin 14 logical px dan offset monitor primer konsisten.

Karena jendela click-through saat idle, `pointerenter`/`pointerleave` dari DOM tidak bisa dipakai. Hover dideteksi dengan polling `cursor_over_island` (posisi kursor OS vs rect pill/panel) tiap 40 ms. Target hover mengikuti apa yang sedang digambar: idle 320×48, notice 460×72, panel 840×248. Panel baru terbuka setelah 3 tick di dalam (120 ms) supaya kursor yang hanya lewat tidak langsung memicu morph; collapse baru jalan setelah 4 tick berturut-turut di luar (160 ms). Setelah 30 detik idle, pill meredup sampai kursor mendekat lagi.

Posisi (kiri/tengah/kanan) menggeser *footprint panel* 840 px, bukan pill-nya, karena layer konten selalu terpusat di jendela; kalau pill yang ditempel ke tepi, panel yang melebar akan terpotong keluar layar.

Panel sekarang memakai tab state (`panelTab`) dan merender satu tab pada satu waktu, bukan track horizontal 400% dengan swipe. Header berisi jam, tanggal, chip jaringan/baterai, gear Setelan, dan tombol tutup. Sparkline menyimpan histori sampel di komponen dan dibatasi jumlahnya; nilai jaringan memakai skala dinamis dari sampel tertinggi agar tetap terbaca saat idle.

Tema memakai token `--di-*` di `app.css` plus alias lama (`--surface`, `--ink`, `--muted`, `--accent`, dst.) dan atribut `data-theme` di `<html>`; komponen tidak menyimpan warna sendiri. Nilai tersimpan dibaca sebelum render pertama agar tema terang tidak berkedip gelap. Motion memakai `--ease-island`/`--ease-pop` tanpa overshoot, durasi sekitar 240 ms, dan menghormati `prefers-reduced-motion`.

HUD volume/tombol adalah lapisan di atas notice unduhan: keduanya punya deadline sendiri (`hudUntil`, `noticeUntil`), HUD menang saat aktif, dan notice unduhan muncul lagi setelah HUD selesai. `announce()` membuang HUD yang masih tampil supaya unduhan yang baru selesai tidak tertutup. Hook keyboard dipasang di thread sendiri yang punya message loop (`WH_KEYBOARD_LL`); auto-repeat ditekan dengan bitmask tombol yang sedang ditahan, jadi menahan Tab tidak membanjiri event.

Permukaan island memakai gradien `--sheen` tipis di tepi atas, drop shadow luar (`--drop`), ring lembut (`--ring`), dan inset glow (`--glow`). Glow dibuat inset karena jendela Tauri transparan berukuran sama dengan island: saat panel penuh, glow luar akan terpotong batas jendela dan tampak kotak.

Tiga hal yang bikin animasi terasa salah dan sudah dihindari: (1) `border-radius` memakai angka piksel (28/36/28), bukan `rounded-full`/9999px; (2) layer `IdleView`/`NoticeView`/`SystemPanel` diberi ukuran piksel tetap dengan `top-0 left-1/2 -translate-x-1/2`, bukan `inset-0`, supaya konten tidak ikut menyusut dan reflow tiap frame; (3) baris media/proses selalu dirender, termasuk saat kosong, supaya panel tidak berubah tinggi ketika sesi media muncul atau hilang.

COM diinisialisasi per thread (`modules/com.rs`) karena Tauri menjalankan command di thread pool-nya; kalau thread itu sudah STA, `CoInitializeEx` menolak mode MTA (`RPC_E_CHANGED_MODE`) dan itu dianggap sukses, bukan error. WinRT async (GSMTC) di-block dengan poll-loop berduri 2 detik, jadi pemutar yang macet tidak parkir thread command selamanya.

Notifikasi Windows dibaca langsung dari `%LOCALAPPDATA%\Microsoft\Windows\Notifications\wpndatabase.db` dengan mode read-only. Tiap 500 ms modul menghitung `MAX("Order")`, lalu mengambil maksimal 8 baris toast baru setelah nilai itu; `<text>` pertama jadi judul dan sisanya digabung dengan " Â· ". Nama app diambil dari `shell:AppsFolder\<AUMID>` lewat Shell COM (beberapa AUMID sistem dipetakan manual karena tidak terdaftar di AppsFolder). Payload tidak disalin atau disimpan â€” event hanya membawa judul, isi dan nama app ke UI. Kalau database dibuat ulang sehingga `Order` turun, watcher melakukan rebase tanpa memutar notifikasi lama.

Pengingat memakai jam dinding lokal (`GetLocalTime` dari API Windows, bukan `SystemTime` UTC) supaya alarm mengikuti hari dan zona waktu pengguna; chrono sengaja tidak ditambahkan hanya untuk membaca tanggal. `wDayOfWeek` Windows (0 = Minggu) dikonversi ke bitmask Senin-first lewat `1 << ((wDayOfWeek + 6) % 7)` agar satu bit = satu tombol chip di UI. Seperti timer dan notifikasi Windows, pemicunya thread `std::thread::spawn` yang `emit` + `reveal`, bukan interval di frontend: pill harus tetap berbunyi saat island tersembunyi atau tidak sedang digambar. CRUD-nya satu kali tulis daftar penuh (`save_reminders` menerima seluruh daftar, backend menormalkan lalu mengembalikan hasilnya) mengikuti pola `set_favorites`, jadi frontend tidak pernah punya id karangan dan tidak perlu sinkronisasi per butir.

Tray memakai icon default aplikasi dan managed state untuk dua item yang berubah saat runtime (label tampil/sembunyi dan checkbox autostart). Autostart memakai plugin resmi dengan entri HKCU Run per-user; command `autostart_status`/`set_autostart` membungkusnya supaya frontend tidak perlu izin plugin tambahan, dan `modules/autostart.rs` mengirim `autostart-changed` agar halaman setelan dan menu tray tidak pernah berbeda state. Saat dev, entri autostart menunjuk `target\debug\desk-island.exe`; setelah aplikasi di-install, toggle ulang supaya menunjuk executable hasil instalasi.

## Roadmap berikutnya

Code signing dan uji installer di mesin bersih.


