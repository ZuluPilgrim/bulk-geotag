# Bulk GeoTag: User Guide

Bulk GeoTag adds a GPS location to every photo in a folder. This guide covers a normal session, every option on the screen, and what to do when something goes wrong.

## Contents

1. [The screen at a glance](#the-screen-at-a-glance)
2. [Tagging a folder, step by step](#tagging-a-folder-step-by-step)
3. [Options explained](#options-explained)
4. [Reading the results](#reading-the-results)
5. [The log file](#the-log-file)
6. [Backups and restoring a photo](#backups-and-restoring-a-photo)
7. [Settings file and place search](#settings-file-and-place-search)
8. [What gets remembered](#what-gets-remembered)
9. [Troubleshooting](#troubleshooting)
10. [Limitations](#limitations)

---

## The screen at a glance

```
┌─────────────────────────┬─────────────────────────────────────┐
│ PHOTOS                  │                                     │
│  Folder  [ … ][Browse…] │                                     │
│  ☐ Include subfolders   │                                     │
│  ☐ Overwrite existing   │              Map                    │
│  ☐ Update modified date │                                     │
│  57 photos · 12 have GPS│   numbered circles = search results │
│ LOCATION                │   red pin = the spot to be written  │
│  [ place name ][Find]   │                                     │
│  ◉ 1 Result one         │                                     │
│  Lat [   ]  Lon [   ]   │                                     │
│ BACKUP                  │                                     │
│  ☐ Back up originals    │                                     │
│  Backup to [ … ][Browse]│                                     │
│ LOG                     │                                     │
│  Write to [ … ][Browse] │                                     │
│ [Preview] [Apply to 45] │                                     │
├─────────────────────────┴─────────────────────────────────────┤
│ Status bar: progress, results, log file name                  │
└───────────────────────────────────────────────────────────────┘
```

- **Blue** buttons pick folders or apply changes. **Green** Preview is a safe dry run.
- Buttons fade out when they can't be used yet. Apply needs a folder, a location, and a backup folder if backup is ticked.

---

## Tagging a folder, step by step

1. **Select the photo folder.** Click **Browse…** next to *Folder*. The app scans it and shows, for example, *57 photos · 12 already have GPS*. Problem files are shown in amber.
2. **Choose the scope.** Tick **Include subfolders** to include every folder below it. Tick **Overwrite existing GPS** only if you want to replace coordinates that are already there. Tick **Update modified date** if the photos are in OneDrive, Dropbox, Google Drive or another synced or backed-up folder (see [File dates](#file-dates-and-sync-tools)).
3. **Set the location.** Type a place in *Location* and press **Enter** or **Find**.
   - If there are several matches, numbered circles appear on the map. Click one in the list or on the map.
   - If there is exactly one match, it is selected automatically.
   - To fine-tune the spot, **drag the red pin** or **click anywhere on the map**. You can also type **Lat** / **Lon** directly and press Tab.
4. **Optional: back up the originals.** Tick **Back up originals** and select a backup folder. It must be **outside** the photo folder.
5. **Optional: keep a log.** Select a folder under **Log**. Click **✕** to turn logging off again.
6. **Preview.** Click the green **Preview** button. A panel lists every photo and what would happen to it. Nothing is changed.
7. **Apply.** Click **Apply to N**, check the confirmation box (coordinates, photo count, folders, whether modified dates will be updated), and click **Write GPS**. A progress bar shows in the status bar. When it finishes, the results panel opens and the status bar shows the totals and the log file name.

> **Tip:** Try it on a folder of *copies* first, until you're comfortable with how it behaves.

---

## Options explained

| Option | Off (default) | On |
|---|---|---|
| **Include subfolders** | Only photos directly in the selected folder | Every folder below it, at any depth |
| **Overwrite existing GPS** | Photos that already have GPS are skipped (logged as a warning) | Their GPS is replaced with the new location |
| **Update modified date** | Each photo keeps its original modified date | Each tagged photo's modified date becomes the time it was tagged, so sync and backup tools notice the change |
| **Back up originals** | No backup | Each photo is copied to the backup folder before it is changed |
| **Log** (folder set) | No log | Errors and warnings are written to a CSV file |

Always skipped, whatever the options:

- **Hidden** files and folders (names starting with `.`)
- **Linked folders** (symbolic links are not followed)
- **PNG** files (writing is not supported yet; logged as a warning)
- **Unreadable** files (corrupt metadata, or not really an image; logged as an error)

---

### File dates and sync tools

Sync and backup tools such as OneDrive, Dropbox, Google Drive and rsync often decide whether a file has changed by looking at its **modified date**. If the date stays the same, they may never upload the tagged photo. **Update modified date** is off by default, and resets to off each time the app starts, so tick it for synced folders.

| Date | Update modified date **on** | **off** |
|---|---|---|
| File *modified* | Time of tagging | Original kept |
| File *accessed* | Time of tagging | Original kept |
| EXIF *Date Taken* / *DateTime* (inside the photo) | Never changed | Never changed |
| Folder *modified* | Changes | Changes |
| *Created* / birth time (e.g. Windows "Date created") | Becomes the time of tagging | Becomes the time of tagging |

Photo apps such as Lightroom, Google Photos, Immich and digiKam sort by EXIF *Date Taken*, which is never touched.

Leave the option **off** for folders that aren't synced, so file managers keep showing the original dates.

---

## Reading the results

The results panel (after Preview or Apply) lists every photo:

| Mark | Meaning |
|---|---|
| ✓ green | Will be written / was written |
| ! amber | Skipped or noted: already has GPS, PNG, or a backup that already existed |
| ✕ red | Error: the photo was **not** changed |

Untick **Show OK files** to see only the problems. The header and status bar show the totals: *written · skipped · warnings · errors*.

A photo with an error is never left half-written. It stays exactly as it was.

---

## The log file

- Written only when a log folder is set **and** there is at least one warning or error.
- Name: `bulk-geotag_YYYY-MM-DD_HH-MM-SS.csv`, using the local time the run started. If a file with that name already exists, new lines are added to the end.
- The full path appears in the status bar after Apply: *Log written: …*
- Preview never writes a log.

Columns:

| Column | Example |
|---|---|
| `time` | `2026-10-10 15:42:28` |
| `level` | `warning` or `error` |
| `folder` | `/home/fred/Photos/Garden` |
| `file` | `IMG_0412.JPG` |
| `issue` | `skipped: already has GPS` |

The file opens directly in LibreOffice Calc or Excel.

---

## Backups and restoring a photo

The backup copy keeps the photo's **full original path** under the backup folder, without the drive letter.

| Photo | Backup folder | Backup copy |
|---|---|---|
| `/home/fred/Photos/2024/a.jpg` | `/media/backup` | `/media/backup/home/fred/Photos/2024/a.jpg` |
| `D:\Photos\2024\a.jpg` | `E:\Backup` | `E:\Backup\Photos\2024\a.jpg` |

Backup copies keep the original photo's modified date.

If a backup copy **already exists**, for example from an earlier run, it is **kept** and the photo is still tagged. The older copy is the more original one. This is logged as a warning.

If the backup copy **fails**, for example because the disk is full, the photo is **not** changed and an error is logged.

**To restore one photo on Linux:**

```bash
cp -p "/media/backup/home/fred/Photos/2024/a.jpg" "/home/fred/Photos/2024/a.jpg"
```

**To restore a whole folder on Linux** (`-a` keeps dates and permissions):

```bash
rsync -a "/media/backup/home/fred/Photos/2024/" "/home/fred/Photos/2024/"
```

---

## Settings file and place search

Place search uses OpenStreetMap's free **Nominatim** service. Its settings are kept in a small file that is created the first time you search:

| System | Location |
|---|---|
| Linux | `~/.config/com.zulupilgrim.bulkgeotag/settings.json` |
| Windows | `%APPDATA%\com.zulupilgrim.bulkgeotag\settings.json` |
| macOS | `~/Library/Application Support/com.zulupilgrim.bulkgeotag/settings.json` |

```json
{
  "nominatim_url": "https://nominatim.openstreetmap.org",
  "contact_email": ""
}
```

- **`contact_email`**: put your email here. It is sent with each search so Nominatim's operators can contact you rather than block the app. It never leaves your computer otherwise.
- **`nominatim_url`**: change this to use a different Nominatim server, such as a self-hosted one.
- Changes take effect on the next search. There's no need to restart.

To follow Nominatim's rules, searches happen only when you press Enter or Find, at most one per second. Results are **cached**, so repeating a search is instant and doesn't contact the server. The cache is stored in:

- Linux: `~/.cache/com.zulupilgrim.bulkgeotag/geocode-cache.json`
- Windows: `%LOCALAPPDATA%\com.zulupilgrim.bulkgeotag\geocode-cache.json`

It is safe to delete.

---

## What gets remembered

Between launches the app remembers:

- the photo folder, which is rescanned on start
- **Back up originals** and the backup folder
- the log folder
- the map's position and zoom

**Not** remembered, on purpose: the location or pin (each batch is usually a different place), the *Include subfolders* and *Overwrite* boxes (safer to tick deliberately each time), and *Update modified date*, which starts **off** at every launch.

---

## Troubleshooting

| Message / symptom | Cause and fix |
|---|---|
| **Apply stays faded** | It needs a folder, a pin, at least one photo to write, and a backup folder if backup is ticked. A scan might also still be running. |
| **"The backup folder is inside the photo folder"** | Select a backup folder outside the photo folder. Otherwise backups would be scanned and tagged next time. |
| **Search failed: … 403** | Nominatim refused the request. Check `contact_email` in settings.json for typos, or try again later. |
| **Search failed: … 429** | Too many requests. Wait a minute. |
| **Search failed: could not reach …** | No internet connection, or `nominatim_url` is wrong. |
| **No places found** | Add a country or region (*Milford Sound, New Zealand*), or click the map instead. |
| **Map tiles are grey** | No internet connection. You can still type Lat/Lon. |
| **✕ "unreadable metadata"** | The file's metadata is damaged or it isn't really an image. It was not changed. |
| **✕ "verification failed"** | The write didn't check out (wrong coordinates read back, or camera data would have changed), so the original was kept. Please report it with the file type and camera model. |
| **TIFF ✕ "requires XResolution"** | Some generated TIFFs lack resolution tags. Camera and scanner TIFFs normally have them. |
| **"Log NOT written: …"** | The log folder is missing or can't be written to. |
| **Folder chooser button says "Open"** | That's normal on Linux. The label comes from the system's file chooser. Clicking it selects the folder. |

---

## Limitations

- PNG, HEIC/HEIF (iPhone) and RAW files are not written yet. PNG is skipped and logged. HEIC and RAW are not scanned at all.
- Only latitude and longitude are written, not altitude or direction.
- There's no undo apart from the backups, so use **Back up originals** for anything you can't afford to lose.
- The development build (`npm run tauri dev`) and the installed app keep separate remembered settings.
