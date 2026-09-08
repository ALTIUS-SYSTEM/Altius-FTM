# Kumpulan Prompt — 17 Lapis Pembangunan Produk

Tujuh belas prompt yang memetakan seluruh lapisan pembangunan produk perangkat lunak, dari keputusan arsitektur sampai penskalaan — termasuk lapisan khusus untuk produk yang memakai LLM saat runtime. Masing-masing dirancang sebagai *system prompt* untuk agen AI, dan bisa dipakai terpisah.

---

## Daftar Berkas

| No | Berkas | Peran |
|---|---|---|
| 01 | `01-system-design.md` | System Designer |
| 02 | `02-system-architecture.md` | System Architect |
| 03 | `03-databases-storage.md` | Database & Storage Engineer |
| 04 | `04-auth-permissions.md` | Auth & Permissions Engineer |
| 05 | `05-security.md` | Security Engineer |
| 06 | `06-ci-cd.md` | CI/CD Engineer |
| 07 | `07-hosting-cloud.md` | Cloud & Hosting Engineer |
| 08 | `08-error-tracking-logs.md` | Logging & Error Tracking Engineer |
| 09 | `09-monitoring-alerts.md` | Monitoring & Alerting Engineer |
| 10 | `10-api-backend-logic.md` | API & Backend Engineer |
| 11 | `11-frontend.md` | Frontend Engineer |
| 12 | `12-testing.md` | Test Engineer |
| 13 | `13-rate-limiting.md` | Rate Limiting Engineer |
| 14 | `14-caching.md` | Caching Engineer |
| 15 | `15-cdn.md` | CDN Engineer |
| 16 | `16-scaling.md` | Scaling Engineer |
| 17 | `17-llm-runtime.md` | AI/LLM Runtime Engineer |

---

## Empat Kategori

Ketujuh belas lapis ini tidak setara bobotnya. Memperlakukannya seolah sama adalah alasan utama orang merasa tumpukan ini mustahil.

**A — Menuntut penilaian Anda (01–04, sisi assertion 12, sisi eval 17)**
System design, architecture, database, dan auth. Tidak bisa didelegasikan penuh karena model bahasa tidak memegang gambaran utuh sistem Anda. Di sini juga masuk dua faset yang menuntut Anda memutuskan apa arti "benar": penulisan assertion (dari 12) dan penetapan eval set beserta baseline (dari 17) — model yang menilai dari implementasi hanya akan mengunci perilaku yang ada, termasuk bugnya. Porsinya kecil, tapi menentukan nasib lapis-lapis sisanya.

**B — Delegasikan, model kuat di sini (10, 11, 13, sisi implementasi 05, sisi scaffolding 12, sisi implementasi 17)**
Setelah kontrak, skema, assertion, dan eval baseline ditetapkan, bagian ini memang cepat dikerjakan: implementasi endpoint dan UI, scaffolding serta fixture test (12), guard injeksi, verifier loop, dan anggaran token (17). Inilah yang membuat pengembangan berbantuan AI nyata.

**C — Pasang sekali, lalu jalan sendiri (06, 07, 08, 09)**
Bukan beban berkelanjutan. Setengah hari sampai dua hari pemasangan, setelah itu bekerja tanpa perlu diingat. Ini jaring pengaman terpenting untuk kode yang diproduksi cepat.

**D — Tunda sampai ada bukti kebutuhan (14, 15, 16)**
Masalah yang muncul kalau produk berhasil. Caching, CDN, dan penskalaan ditambahkan saat angka performa menuntutnya — masing-masing dokumen ini bahkan melarang dirinya dipasang tanpa bukti beban nyata. Membangunnya di awal adalah perluasan cakupan yang menyamar sebagai profesionalisme.

---

## Urutan Pemakaian

Kerjakan **A → C → B → D**.

Tetapkan keputusan dulu, pasang jaring pengaman, baru produksi kode cepat, dan tunda dua lapis terakhir sampai angka menuntutnya.

Kebanyakan orang memulai dari B. Itulah sebabnya tumpukan ini terasa berat — lapis-lapis sisanya jadi berlipat sulitnya ketika empat lapis pertama dilewati.

---

## Cara Memakai

**Sebagai system prompt.** Tempel isi satu berkas ke Claude Project, Custom GPT, atau berkas instruksi agen coding. Satu berkas per sesi — jangan digabung, karena konteks yang terlalu luas membuat model melupakan aturan spesifik.

**Sebagai konteks tetap di repositori.** Simpan berkas yang relevan di `docs/prompts/` dan rujuk dari `CLAUDE.md`. Konteks yang hidup di berkas akan konsisten di semua sesi; konteks yang hidup di riwayat percakapan akan hilang.

**Sebagai daftar periksa tinjauan.** Bagian Pemeriksaan Mandiri di tiap berkas bisa dipakai langsung untuk meninjau pekerjaan yang sudah jadi, tanpa menjalankan prompt lengkapnya.

---

## Aturan Antar-Dokumen

Setiap prompt menyebutkan masukan yang dibutuhkannya dari tahap sebelumnya. Tiga aturan yang berlaku di semuanya:

1. **Keputusan tahap sebelumnya tidak boleh diubah sepihak.** Kalau bermasalah, laporkan dan berhenti.
2. **Invariant dari dokumen desain harus punya tempat penegakan di setiap lapisan yang menyentuhnya.** Tiap invariant diberi ID stabil di `01` (`INV-1`, `INV-2`, …); `03`, `10`, dan `12` merujuk ID itu di tabelnya masing-masing, sehingga invariant tanpa baris penegakan bisa terdeteksi tanpa membaca semua dokumen.
3. **Tidak ada komponen yang ditambahkan tanpa requirement yang menuntutnya.**
