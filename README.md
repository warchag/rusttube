# RustTube — YouTube Studio Downloader

โปรแกรมดาวน์โหลด YouTube พร้อม GUI สไตล์โมเดิร์น เขียนด้วยภาษา **Rust** ล้วน (`eframe` / `egui`) รองรับภาษาไทย/อังกฤษ เต็มรูปแบบ พร้อมเครื่องมือจัดการไฟล์สื่อด้วย **FFmpeg** ในตัว

---

## ฟีเจอร์ทั้งหมด (Features)

### 1. ดาวน์โหลดวิดีโอ & เสียง (Downloader)
- **Smart Paste & Rich Preview**: วางลิงก์จาก Clipboard พร้อมดึงข้อมูลวิดีโออัตโนมัติ แสดงรูปปก (Thumbnail), ชื่อวิดีโอ (รองรับฟอนต์ภาษาไทย), ชื่อช่อง, ความยาว, ยอดวิว, วันที่อัปโหลด และขนาดไฟล์โดยประมาณของแต่ละความละเอียด
- **Video + Audio**: เลือกความละเอียดได้ตั้งแต่ `Best`, `4K (2160p)`, `2K (1440p)`, `Full HD (1080p)`, `HD (720p)`, `SD (480p)`, `360p` รองรับนามสกุล `MP4`, `MKV`, `WEBM`
- **Codec Selector**: เลือกบังคับ Codec ได้ (`Auto`, `H.264` สำหรับ iPhone/QuickTime/ตัดต่อ, `VP9`, `AV1`)
- **Audio Only**: แยกเสียงคุณภาพสตูดิโอเป็น `MP3`, `M4A`, `FLAC`, `WAV`, `OPUS` พร้อมเลือก Bitrate (`320 / 192 / 128 kbps`)
- **Quick Preset Profiles**: ปุ่มลัดตั้งค่าด่วนในคลิกเดียว
  - `🎵 Music MP3 320k`
  - `📱 iPhone MP4 (H.264)`
  - `🎬 4K Master`
  - `🎙 Podcast` (M4A + SponsorBlock)
  - `✂ Quick Clip` (ตัดเฉพาะช่วงเวลา)

### 2. โหมดขั้นสูง & คิวอัจฉริยะ (Pro Options & Queue)
- **Batch Download (หลายลิงก์พร้อมกัน)**: วาง URL ทีละหลายบรรทัด หรือกดนำเข้าจากไฟล์ `.txt` แล้วเพิ่มเข้าคิวทั้งหมดในคลิกเดียว
- **Playlist Picker**: ดึงรายการคลิปในเพลย์ลิสต์ทั้งหมดพร้อม Checkbox เลือกโหลดเฉพาะคลิปที่ต้องการ
- **Pro Options**:
  - **Clip Range**: ตัดโหลดเฉพาะช่วงเวลาที่ต้องการ (`Start` – `End`)
  - **SponsorBlock**: ข้ามและตัดช่วงโฆษณา/สปอนเซอร์อัตโนมัติ
  - **Subtitles**: โหลดซับไตเติล (ตั้งภาษาได้ เช่น `th,en`) และฝังลงวิดีโอ
  - **Embed Thumbnail & Metadata**: ฝังรูปปกและข้อมูลเพลง/ศิลปินลงไฟล์อัตโนมัติ
  - **Speed Limit**: จำกัดความเร็วเน็ต (`Unlimited`, `10 / 5 / 2 / 1 MB/s`)
  - **Browser Cookies**: ใช้ Cookie จาก `Chrome`, `Safari`, `Firefox`, `Brave`, `Edge` สำหรับคลิปจำกัดอายุหรือต้องล็อกอิน
- **Live Progress Queue**: แสดงแถบสถานะเรียลไทม์ (`%`, ความเร็ว `MiB/s`, `ETA`, ขนาดไฟล์, ขั้นตอน Merging/Extracting) พร้อม Animation ระหว่างเชื่อมต่อ ปุ่ม **Cancel**, **Retry**, **Play** และ **Reveal in Finder**
- **Clipboard Watcher**: โหมดตรวจจับลิงก์ YouTube จาก Clipboard อัตโนมัติ
- **macOS Native Notification**: แจ้งเตือนผ่าน macOS Notification Center พร้อมเสียงเมื่อดาวน์โหลดหรือแปลงไฟล์เสร็จ

### 3. เครื่องมือไฟล์สื่อในตัว (`🛠 Media Tools` / FFmpeg)
รองรับ **Drag & Drop** ลากไฟล์จาก Finder มาวางในหน้าต่างแอปได้ทันที:
- **Convert Format**: แปลงเป็น `MP4 (H.264)`, `MKV`, `WEBM`, `MP3 (320k)`, `M4A`, `WAV`, `FLAC`, `GIF`
- **Compress Video**: บีบอัดลดขนาดไฟล์ (ปรับ CRF 20–32 และย่อความละเอียด `1080p / 720p / 480p`)
- **Trim / Cut**: ตัดช่วงเวลาวิดีโอ (รองรับ Fast Stream Copy แบบเร็วทันทีไม่เสียคุณภาพ)
- **Extract Audio**: แยกเสียงออกจากวิดีโอเป็น `MP3 / M4A / FLAC / WAV / OPUS`

### 4. ธีม 2 ภาษา & การจัดการระบบ (Themes, i18n & Diagnostics)
- **8 ธีมสวยๆ + แกลเลอรีพรีวิว**:
  - 🌑 `Midnight Studio` | 🌃 `Tokyo Night` | ☕ `Catppuccin Mocha` | ⚡ `Cyberpunk Neon`
  - ❄ `Nordic Frost` | 🧛 `Dracula Velvet` | 🌸 `Sakura Blossom` | ☀ `Apple Snow`
  - พร้อม **Accent Override** 7 สี และปุ่มสลับธีมด่วนที่แถบด้านซ้าย
- **ระบบเปลี่ยนภาษา ไทย / English**: สลับได้ทันทีจากปุ่ม `🌐 ไทย / EN` ที่แถบด้านซ้ายหรือหน้า Settings
- **History & Persistent Config**: บันทึกการตั้งค่าและประวัติการดาวน์โหลดอัตโนมัติ (`~/Library/Application Support/RustTube/`)
- **Smart Engine Detection & One-Click Update**: ตรวจหา `yt-dlp` เวอร์ชันใหม่ที่สุดในเครื่องอัตโนมัติ พร้อมปุ่ม **Update yt-dlp** และหน้าต่าง **Console Logs**

---

## ความต้องการของระบบ (Requirements)

- **Rust Toolchain** (`rustc` / `cargo`) — [rustup.rs](https://rustup.rs)
- **yt-dlp** — `brew install yt-dlp` หรือ `pip install -U yt-dlp`
- **ffmpeg** — `brew install ffmpeg`

---

## วิธีรันและ Build

### รันจากโค้ด (Development / Release)
```bash
# โหมดพัฒนา (คอมไพล์เร็ว)
cargo run

# โหมด Release (เร็วที่สุด)
cargo run --release
```

### รัน Unit Tests
```bash
cargo test
```

### Build เป็นแอป macOS (`.app` + `.dmg`)
```bash
./build_macos_app.sh
```
จะได้ไฟล์พร้อมใช้งานในโฟลเดอร์ `dist/`:
- `dist/RustTube.app` — ดับเบิลคลิกเปิดใช้งานได้ทันที หรือรัน `open "dist/RustTube.app"`
- `dist/RustTube-1.0.0-macOS.dmg` — ไฟล์ติดตั้งสำหรับลากเข้า **Applications**

---

## โครงสร้างโปรเจกต์ (Project Structure)

| ไฟล์ | หน้าที่ |
| :--- | :--- |
| `Cargo.toml` | ตั้งค่าแพ็กเกจและ Dependencies (`eframe`, `reqwest`, `image`, `rfd`, `arboard`, `serde`, `chrono`, `uuid`) |
| `src/main.rs` | จุดเริ่มต้นโปรแกรมและตั้งค่าหน้าต่างแอป |
| `src/models.rs` | โครงสร้างข้อมูลทั้งหมด (Config, Theme, Language, Download Options, Queue, History, Media Tools) |
| `src/ui.rs` | ส่วนติดต่อผู้ใช้ GUI ครบทั้ง 4 แท็บ (`Downloader`, `Media Tools`, `History`, `Settings & Logs`) |
| `src/ytdlp.rs` | ระบบค้นหา `yt-dlp` เวอร์ชันล่าสุด, ดึงข้อมูล/ขนาดไฟล์/Playlist, อ่าน Progress เรียลไทม์, อัปเดต `yt-dlp` และแจ้งเตือน macOS |
| `src/ffmpeg_tools.rs` | ระบบแปลงไฟล์, บีบอัดวิดีโอ, ตัดช่วงเวลา และแยกเสียงด้วย `ffmpeg` |
| `src/theme.rs` | ชุดสี 8 ธีม + 7 Accent Colors และการโหลดฟอนต์ macOS + ภาษาไทย |
| `src/i18n.rs` | ระบบแปลภาษาไทย / อังกฤษ (`t(lang, en, th)`) |
| `src/config.rs` | โหลดและบันทึกไฟล์ตั้งค่า (`config.json`) และประวัติ (`history.json`) |
| `build_macos_app.sh` | สคริปต์ Build Release, สร้างไอคอน `.icns`, แพ็ก `RustTube.app`, Code Sign และสร้าง `.dmg` |

---

## การแก้ปัญหาเบื้องต้น (Troubleshooting)

- **เจอ `HTTP Error 403: Forbidden` ตอนดาวน์โหลด**:
  - เปิดแท็บ **Settings & Logs** แล้วกดปุ่ม **⬆ อัปเดต yt-dlp** (หรือรัน `pip install -U yt-dlp` / `brew upgrade yt-dlp`) แล้วกด **↻ ตรวจสอบใหม่**
  - สำหรับคลิปที่จำกัดอายุหรือต้องล็อกอิน ให้เลือก **Browser Cookies** ใน **ตัวเลือกเสริม (Pro Options)**
- **ไม่มีเสียงหรือรวมไฟล์ไม่ได้**:
  - ตรวจสอบในแถบซ้ายล่างว่า `ffmpeg` แสดงจุดสีเขียว หากยังไม่ติดตั้งให้รัน `brew install ffmpeg`
