# Changelog

All notable changes to Bulk GeoTag. Dates are NZ local time.
Format based on [Keep a Changelog](https://keepachangelog.com/); versions follow [Semantic Versioning](https://semver.org/).

## [1.4.0] - 2026-10-15

### Added
- **Drag and drop:** drop a folder onto the window to switch to *Bulk folder update* and scan it, or one JPEG/TIFF/WebP photo to switch to *Single file update* and open it. A dashed outline shows while dragging; several items, PNGs and other files get a message in the status bar instead. Handy on Linux, where the system's Browse… window has no thumbnail view.
- **Photo details card on the map** (single-file mode): the preview and the photo's GPS, date taken, camera, lens, exposure · aperture · ISO, focal length (with 35 mm equivalent), size and software, in the map's top-right corner. ▾/▸ folds it; the choice is remembered.

### Changed
- Single-file mode: the small photo card in the left panel is replaced by the card on the map.
- User Guide: new *Drag and drop* section, the details card, and troubleshooting entries for drops and the Browse… window.

## [1.3.1] - 2026-10-15

### Fixed
- Photos whose EXIF stores a standard tag with the wrong number type (for example darktable exports, which write XResolution as a whole number) were left unchanged with *cannot load existing metadata safely: Illegal format for known tag*. These now tag normally; the affected tags keep their values and are stored with the correct type. Nothing else in the EXIF changes.
- Help in `npm run tauri dev`: the User Guide and its screenshot were blocked by Vite's file-serving rules. Installed builds were not affected.

## [1.3.0] - 2026-10-11

### Added
- **In-app help:** the User Guide opens inside the app with the **?** button (next to ⚙) or **F1**. It works offline: the guide and its pictures are built into the app. Contents links jump within the guide; web links open in the system browser. Esc, ✕ or clicking outside closes it.
- Screenshot in the User Guide's *The screen at a glance*, replacing the text diagram. The README uses the same, updated screenshot.
- `npm run check:guide`: checks that every Contents link in the User Guide matches a heading and every image exists.

### Changed
- User Guide: *File dates and sync tools* now explains how photo apps order their timeline (EXIF date taken) and how OneDrive and other sync tools notice changes (file modified date), with a *Which to use* table.

## [1.2.0] - 2026-10-10

### Added
- **⚙ Settings** dialog (top-right of the left panel): contact email (with a format warning), Nominatim server with *Reset*, search language codes, results per search (1–10), *Clear cache* with a count, and *Show in folder* for settings.json. Esc or Cancel closes without saving.
- `language` and `max_results` in settings.json (defaults `en` and 5); older settings files load unchanged.
- Settings are validated before saving.

### Changed
- Search language and result count come from settings instead of being fixed; both are part of the search cache key.
- 403 message now points to ⚙ Settings.

### Packaging
- Linux packages: `.deb`, `.rpm` and AppImage via `npm run tauri build`.
- App name "Bulk GeoTag" in menus (category Graphics); command stays `bulk-geotag`.
- New app icon (photo with a map pin); source in `app-icon.svg`.
- Windows installers (`setup.exe` and `.msi`, x64), built by GitHub Actions and attached to the release. Unsigned for now.
- macOS universal `.dmg` (Apple Silicon + Intel), built by GitHub Actions. **Not tested on a Mac** (no Mac hardware available); not signed or notarised.
- Packages published as a GitHub Release with `SHA256SUMS.txt`.

## [1.1.1] - 2026-10-10

### Added
- This changelog.
- Code comments for every function in the UI (`+page.svelte`) and the place-search client (`geocode.rs`).

No change in behaviour.

## [1.1.0] - 2026-10-10 (git tag `v1.1`)

### Added
- **Single file update** mode alongside **Bulk folder update** (radio buttons at the top of *Photos*; the last mode is remembered).
- Photo card in single-file mode: preview, file name, camera, current date taken and GPS.
- Grey pin at the photo's current location; the pin turns red when moved, with a grey marker left at the old spot.
- Lat/Lon and date taken pre-filled from the photo, with *"Already set on this photo. Tick Overwrite … to change it."* hints.
- `inspect_photo` command (`inspect.rs`) and single-file support in scan and apply.

### Changed
- App version set to 1.1.0 (`package.json`, `Cargo.toml`, `tauri.conf.json`); the Nominatim User-Agent now reports it.

## [1.0.0] - 2026-10-10 (git tag `mvp-1.0`)

First complete version.

### Added
- Folder scan (optional subfolders, any depth) with counts of photos, existing GPS and unreadable files.
- Place search via OpenStreetMap Nominatim (rate-limited, cached, contact email from `settings.json`), map pin, typed coordinates.
- Optional **date taken**: one date and time for all photos, written to DateTimeOriginal and CreateDate, with its own *Overwrite existing date taken* option. Date picker plus typed time (blank = 12:00 noon).
- EXIF *Modified* stamp (DateTime) set to the time of any EXIF change.
- Preview (dry run) and Apply with confirmation, progress bar and results panel.
- Safe writes: temp copy, verification, atomic replace; camera data preserved.
- **Update file modified date** option (off by default), so sync tools such as OneDrive can upload changed photos.
- Optional backups mirroring each photo's full path; backups keep the original file dates; existing backups are kept.
- Optional CSV log of warnings and errors, named by run time, appended if it already exists.
- Remembered photo, backup and log folders, backup setting and map position.
- README, User Guide and Architecture docs.

### Known limitations
- PNG is scanned but not written; HEIC and RAW are not supported.
- Windows installers are unsigned (SmartScreen warning on first run).
- macOS build is untested and not notarised (Gatekeeper blocks it until allowed).
- On Linux the folder chooser's button reads "Open" (set by the system dialog).
