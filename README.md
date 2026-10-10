# Bulk GeoTag

A desktop app for adding GPS location and/or a date taken to a whole folder of photos at once, or to a single photo.

Select a folder, type a place name (or click the map), confirm the spot, optionally set a date taken, and Bulk GeoTag writes them into every photo's EXIF data. It can back up the originals first and log anything it had to skip.

Built with [Tauri 2](https://tauri.app) (Rust) and [Svelte](https://svelte.dev), with maps and place search from [OpenStreetMap](https://www.openstreetmap.org).

## Features

- **Two modes**: *Bulk folder update* or *Single file update*. Single-file mode shows the photo, its current GPS (grey pin) and date taken, and pre-fills them ready to adjust.
- **Folder scan**: counts photos and shows how many already have GPS. Can include subfolders (any depth).
- **Place search**: type a name such as *Colosseum, Rome* and pick from up to 5 matches, or click/drag the pin to an exact spot. Coordinates can also be typed in.
- **Date taken** (optional): set one date and time for every photo, with its own *overwrite existing* option. Can be used with or without a location.
- **Preview**: a dry run that lists what would happen to every photo, without changing anything.
- **Safe writing**: each photo is written to a temporary copy, checked, and only then swapped in. Camera data (make, model, orientation) is kept, and the date taken is only changed if you set one. The EXIF *Modified* stamp is set to the time of the change. The file's modified date is kept by default. Tick *Update file modified date* to change it, so OneDrive and other sync tools upload the tagged photos.
- **Optional backups**: originals are copied first into a backup folder that mirrors their full path.
- **Optional CSV log**: errors and warnings (skipped files, backup notices) go to a timestamped log file.
- **Remembers** the last mode, the photo, backup and log folders, and the map position.

| Format | Read GPS | Write GPS |
|---|---|---|
| JPEG (`.jpg`, `.jpeg`) | ✓ | ✓ |
| TIFF (`.tif`, `.tiff`) | ✓ | ✓ |
| WebP (`.webp`) | ✓ | ✓ |
| PNG (`.png`) | ✓ | skipped and logged (not supported yet) |

## Documentation

- [User Guide](docs/USER_GUIDE.md): how to use the app
- [Architecture](docs/ARCHITECTURE.md): how it is built, for developers

## Quick start (development)

Prerequisites on Ubuntu / Debian / Linux Mint 22+:

```bash
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y   # Rust
# Node.js LTS, e.g. via nvm: https://github.com/nvm-sh/nvm
```

Run:

```bash
npm install
npm run tauri dev
```

Tests:

```bash
npm run check                       # TypeScript / Svelte
cd src-tauri && cargo test --lib    # Rust unit tests
```

For other platforms, see the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

## Credits

- Map data and place search © [OpenStreetMap contributors](https://www.openstreetmap.org/copyright), via the [Nominatim](https://nominatim.org) service. The app follows the [Nominatim usage policy](https://operations.osmfoundation.org/policies/nominatim/).
- Maps drawn with [Leaflet](https://leafletjs.com).
- EXIF reading: [kamadak-exif](https://crates.io/crates/kamadak-exif). EXIF writing: [little_exif](https://crates.io/crates/little_exif).
