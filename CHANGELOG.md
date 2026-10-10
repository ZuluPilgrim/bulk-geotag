# Changelog

All notable changes to Bulk GeoTag. Dates are NZ local time.
Format based on [Keep a Changelog](https://keepachangelog.com/); versions follow [Semantic Versioning](https://semver.org/).

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
