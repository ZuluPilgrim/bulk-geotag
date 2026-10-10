# Releasing a new version

How to publish a new version of Bulk GeoTag with packages for Linux, Windows and macOS.

The same steps are also saved as a **Claude Skill** (`bulk-geotag-release`). With it, you can ask Claude *"release Bulk GeoTag 1.3.0"* and it walks you through each step below, checking the terminal output as you go.

## Overview

| Platform | Built by | When |
|---|---|---|
| Linux: `.deb`, `.rpm`, AppImage | You, locally (`npm run tauri build`) | Before tagging |
| Windows: `setup.exe`, `.msi` | GitHub Actions (`.github/workflows/release-windows.yml`) | Automatically when a `v*` tag is pushed |
| macOS: universal `.dmg` | GitHub Actions (`.github/workflows/release-macos.yml`) | Automatically when a `v*` tag is pushed |

Both workflows upload to the tag's GitHub Release and rename their files from `Bulk.GeoTag_*` to `bulk-geotag_*`. The Linux packages and `SHA256SUMS.txt` are uploaded by hand afterwards, so there's no race with the workflows creating the release.

The version number lives in **three** files: `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.

## Steps

Set the new and previous versions once, then run each step in order, checking the output before moving on:

```bash
cd ~/dev/bulk-geotag
V=1.3.0; OLD=1.2.0
```

### 1. Pre-flight

```bash
git status -sb
git pull
npm run check 2>&1 | tail -1
(cd src-tauri && cargo test --lib 2>&1 | tail -4)
```

Expect a clean working tree, `0 ERRORS`, and `test result: ok`.

### 2. Bump the version

```bash
node -e 'const fs=require("fs");for(const f of ["package.json","src-tauri/tauri.conf.json"]){const j=JSON.parse(fs.readFileSync(f));j.version=process.argv[1];fs.writeFileSync(f,JSON.stringify(j,null,2)+"\n")}' "$V"
sed -i "0,/^version = \"$OLD\"/s//version = \"$V\"/" src-tauri/Cargo.toml
grep -m1 '"version"' package.json; grep -m1 '"version"' src-tauri/tauri.conf.json; grep -m1 '^version' src-tauri/Cargo.toml
```

All three lines must show the new version.

### 3. Update the changelog

Add a `## [<version>] - <date>` section at the top of `CHANGELOG.md`, with *Added*, *Changed* and *Fixed* as needed.

### 4. Build the Linux packages (about 5–10 minutes)

```bash
npm run tauri build 2>&1 | tail -8
```

Expect `Finished 3 bundles` (deb, appimage, rpm).

### 5. Commit, tag and push

Pushing the tag starts the Windows and macOS builds.

```bash
git add -A && git status --short
git commit -m "v$V: <summary>"
git tag "v$V"
git push origin main "v$V"
```

The push output must include `* [new tag]`.

### 6. Wait for the Windows and macOS builds

```bash
gh run list --limit 2 --json name,status,conclusion,createdAt,updatedAt
```

Repeat until both *Release (Windows)* and *Release (macOS)* show `completed` and `success`. First runs took about 17 min (Windows) and 10 min (macOS); later runs are faster thanks to the Rust cache. If one fails:

```bash
gh run view <run-id> --log-failed | tail -40
```

### 7. Upload the Linux packages

Copy them without spaces in their names, then upload:

```bash
B=src-tauri/target/release/bundle; mkdir -p /tmp/bgrel && rm -f /tmp/bgrel/*
cp "$B/deb/Bulk GeoTag_${V}_amd64.deb"           /tmp/bgrel/bulk-geotag_${V}_amd64.deb
cp "$B/rpm/Bulk GeoTag-${V}-1.x86_64.rpm"        /tmp/bgrel/bulk-geotag-${V}-1.x86_64.rpm
cp "$B/appimage/Bulk GeoTag_${V}_amd64.AppImage" /tmp/bgrel/bulk-geotag_${V}_amd64.AppImage
gh release upload "v$V" /tmp/bgrel/*
```

### 8. Regenerate the checksums

```bash
rm -rf /tmp/bgsum && mkdir /tmp/bgsum && cd /tmp/bgsum
gh release download "v$V" -R ZuluPilgrim/bulk-geotag -p 'bulk-geotag*'
sha256sum bulk-geotag* > SHA256SUMS.txt && cat SHA256SUMS.txt
gh release upload "v$V" SHA256SUMS.txt -R ZuluPilgrim/bulk-geotag --clobber
cd ~/dev/bulk-geotag
```

Expect 6 lines: deb, rpm, AppImage, setup.exe, msi and dmg.

### 9. Release title and notes

Always keep the Windows *unsigned* and macOS *untested* notices:

```bash
gh release edit "v$V" --title "Bulk GeoTag $V" --notes "Bulk GeoTag $V for Linux, Windows and macOS.

<summary of changes>

Linux: .deb (Mint/Ubuntu/Debian), .rpm (Fedora/openSUSE) or AppImage (any distro).
Windows 10/11: bulk-geotag_${V}_x64-setup.exe (or the .msi). Not code-signed: if SmartScreen says 'Windows protected your PC', click More info, then Run anyway.
macOS (Apple Silicon and Intel): bulk-geotag_${V}_universal.dmg. NOT TESTED on a Mac (no Mac hardware available). Not signed or notarised: right-click the app, then Open, or run: xattr -cr \"/Applications/Bulk GeoTag.app\"

Verify downloads with SHA256SUMS.txt. See CHANGELOG.md for details."
gh release view "v$V" --json assets -q '.assets[].name'
```

### 10. Verify

- Install the `.deb` (`sudo apt install /tmp/bgrel/bulk-geotag_${V}_amd64.deb`) and launch it from the menu.
- If possible, install the Windows `setup.exe` on a Windows machine or VM.
- Optionally record the build times in [DEVELOPMENT_TIME.md](DEVELOPMENT_TIME.md).

## Notes

- **Unsigned Windows installers:** SmartScreen warns on first run. Code-signing needs a paid certificate.
- **macOS:** the build has **not been tested on a Mac**. It isn't signed or notarised, so Gatekeeper blocks it until the user allows it.
- **Public repo:** GitHub Actions builds on standard runners are free.
- To publish to an **existing** tag instead (rebuild Windows/macOS for it), run `gh workflow run "Release (Windows)" -f tag=<tag>` and `gh workflow run "Release (macOS)" -f tag=<tag>`.
