# Bulk GeoTag: User Guide

Bulk GeoTag adds a GPS location and/or a date taken to every photo in a folder. This guide covers a normal session, every option on the screen, and what to do when something goes wrong.

## Contents

1. [The screen at a glance](#the-screen-at-a-glance)
2. [Tagging a folder, step by step](#tagging-a-folder-step-by-step)
   - [Single file update](#single-file-update)
   - [Drag and drop](#drag-and-drop): drop a folder or a photo from your file manager
3. [Options explained](#options-explained)
   - [File dates and sync tools](#file-dates-and-sync-tools): which dates photo apps and sync tools use
   - [Setting the date taken](#setting-the-date-taken)
4. [Reading the results](#reading-the-results)
5. [The log file](#the-log-file)
6. [Backups and restoring a photo](#backups-and-restoring-a-photo)
7. [Settings and place search](#settings-and-place-search)
8. [What gets remembered](#what-gets-remembered)
9. [Troubleshooting](#troubleshooting)
10. [Limitations](#limitations)

---

## The screen at a glance

![Bulk GeoTag: photo folder, place search, date taken, backup and log on the left; map with the search results on the right](images/screenshot.png)

- **Left panel**, top to bottom: **Photos** (mode, folder, scope options and the photo count), **Location** (place search and its results, Lat/Lon), **Date taken**, **Backup**, **Log**, then **Preview** and **Apply** at the bottom (scroll down if the window is short).
- **?** and **⚙** at the top-right of the left panel open this guide (or press **F1**) and the [Settings](#settings-and-place-search).
- **Map** on the right: numbered circles are search results, and the red pin is the spot that will be written. In single-file mode, the photo and its details show in a card at the map's top-right corner.
- **Drag and drop:** drop a folder or a photo anywhere on the window instead of using Browse… (see [Drag and drop](#drag-and-drop)).
- **Status bar** along the bottom: progress, results and the log file name.
- **Blue** buttons pick folders or apply changes. **Green** Preview is a safe dry run.
- Buttons fade out when they can't be used yet. Apply needs a folder, a location and/or a date taken, and a backup folder if backup is ticked.

---

## Tagging a folder, step by step

1. **Select the photo folder.** Click **Browse…** next to *Folder*, or [drag the folder onto the window](#drag-and-drop) from your file manager. The app scans it and shows, for example, *57 photos · 12 already have GPS*. Problem files are shown in amber.
2. **Choose the scope.** Tick **Include subfolders** to include every folder below it. Tick **Overwrite existing GPS** only if you want to replace coordinates that are already there. Tick **Update file modified date** if the photos are in OneDrive, Dropbox, Google Drive or another synced or backed-up folder (see [File dates](#file-dates-and-sync-tools)).
3. **Set the location.** Type a place in *Location* and press **Enter** or **Find**.
   - If there are several matches, numbered circles appear on the map. Click one in the list or on the map.
   - If there is exactly one match, it is selected automatically.
   - To fine-tune the spot, **drag the red pin** or **click anywhere on the map**. You can also type **Lat** / **Lon** directly and press Tab.
   - To change only the date (no GPS), leave the location empty, or click **✕** next to Lat/Lon to clear it.
   - **Optional: set the date taken.** Pick a date from the calendar under **Date taken** and type a time next to it (blank = 12:00 noon). Every photo gets that same date and time. Tick **Overwrite existing date taken** to replace a date the camera already recorded; otherwise only photos without one get it. See [Setting the date taken](#setting-the-date-taken).
4. **Optional: back up the originals.** Tick **Back up originals** and select a backup folder. It must be **outside** the photo folder.
5. **Optional: keep a log.** Select a folder under **Log**. Click **✕** to turn logging off again.
6. **Preview.** Click the green **Preview** button. A panel lists every photo and what would happen to it. Nothing is changed.
7. **Apply.** Click **Apply to N**, check the confirmation box (photo count, location, date taken, overwrite options, folders), and click **Write**. A progress bar shows in the status bar. When it finishes, the results panel opens and the status bar shows the totals and the log file name.

> **Tip:** Try it on a folder of *copies* first, until you're comfortable with how it behaves.

---

### Single file update

Choose **Single file update** at the top of *Photos* to work on one photo instead of a folder.

1. Click **Browse…** and pick a photo (`.jpg .jpeg .tif .tiff .webp`), or [drag the photo onto the window](#drag-and-drop). *Include subfolders* is hidden in this mode.
2. A **photo details card** appears in the **top-right corner of the map**: a preview, the file name, and what the photo already holds: **GPS**, **date taken**, **camera**, **lens**, **exposure · aperture · ISO**, **focal length** (with the 35 mm equivalent), **size** in pixels and the **software** that last saved it. Details the photo doesn't have are left out. Click **▾** to fold the card down to its title bar (and **▸** to open it again); the app remembers which you chose.
3. If the photo already has GPS, the map zooms there and the pin is **grey**: that's where the photo is tagged now. **Lat/Lon** and **Date taken** are pre-filled with the photo's current values.
4. Drag the pin (or click the map, or search) to a new spot. It turns **red**, and a small grey marker stays at the photo's current location for reference. Move it back and it turns grey again.
5. Values the photo already has are protected like in bulk mode. A hint under the field says *"Already set on this photo. Tick Overwrite … to change it."* Until you tick the matching **Overwrite** box, *Apply* counts 0.
6. **Preview** / **Apply** work as normal. After Apply, the details card and grey pin update to the new values.

Backups, the log and *Update file modified date* work exactly as in bulk mode. The backup folder can't be inside the photo's own folder.

> TIFF photos show *No preview* unless they contain a small embedded thumbnail; tagging them works normally.

---

### Drag and drop

Instead of **Browse…**, you can drag from your file manager (Nemo, Files, Dolphin, Windows Explorer or the macOS Finder) and drop anywhere on the Bulk GeoTag window. While you drag, the window shows a dashed outline.

| You drop | What happens |
|---|---|
| **A folder** | Switches to **Bulk folder update**, selects that folder and scans it, as if you had chosen it with Browse… |
| **One photo** (`.jpg .jpeg .tif .tiff .webp`) | Switches to **Single file update** and opens that photo, with its details card on the map |
| **Several items at once** | Nothing changes; the status bar says *Drop one folder or one photo at a time.* |
| **A PNG or another kind of file** | Nothing changes; the status bar says which types can be tagged |

Drops are ignored while **Preview** or **Apply** is running, and while Help or Settings is open. The location, date taken and other options stay as they were.

> **Tip:** On Linux Mint and other GTK desktops, the **Browse…** window can only show a list of names, not thumbnails. To pick photos by sight, browse in your file manager's icon or thumbnail view and drag the folder or photo across.

---

## Options explained

| Option | Off (default) | On |
|---|---|---|
| **Include subfolders** | Only photos directly in the selected folder | Every folder below it, at any depth |
| **Overwrite existing GPS** | Photos that already have GPS keep it (logged as a warning) | Their GPS is replaced with the new location |
| **Overwrite existing date taken** | Photos that already have a date taken keep it (logged as a warning) | Their date taken is replaced with the one you set |
| **Update file modified date** | Each photo keeps its original modified date | Each tagged photo's modified date becomes the time it was tagged, so sync and backup tools notice the change |
| **Back up originals** | No backup | Each photo is copied to the backup folder before it is changed |
| **Log** (folder set) | No log | Errors and warnings are written to a CSV file |

Always skipped, whatever the options:

- **Hidden** files and folders (names starting with `.`)
- **Linked folders** (symbolic links are not followed)
- **PNG** files (writing is not supported yet; logged as a warning)
- **Unreadable** files (corrupt metadata, or not really an image; logged as an error)

---

### File dates and sync tools

A photo has two separate sets of dates, and different programs look at different ones:

- **EXIF dates** are stored **inside** the photo, usually by the camera: *Date Taken* (DateTimeOriginal), *Digitized* (CreateDate) and *Modified* (DateTime). They travel with the photo wherever it is copied, emailed or uploaded.
- **File dates** are kept by the computer's file system, **outside** the photo: *modified*, *accessed* and *created*. File managers show these, and they can change when a file is copied, saved or synced.

Bulk GeoTag writes the EXIF *Date Taken* only if you set one. It changes the file *modified* date only if **Update file modified date** is ticked.

#### How photo apps order the timeline

Photo managers such as Google Photos, Apple Photos, Windows Photos, OneDrive's Photos view, Lightroom, Immich and digiKam place each photo on their timeline by its EXIF **Date Taken**. When a photo has one, its file dates don't matter. So:

- **Setting a date taken** is what moves a photo to the right place on the timeline.
- **Updating the file modified date does not** move a photo that has a date taken.
- Photos **without** a date taken (scanned prints, screenshots, pictures saved from the web or messaging apps) fall back to a file date or the upload date in most apps. That's why such photos often turn up on the wrong day, for example the day they were copied. Setting a date taken fixes them in place.
- Many apps read the date and location **once, when the photo is first added**. A photo that was already in the library may need a rescan or re-import before the change shows. For example, in Lightroom Classic use *Metadata → Read Metadata from Files*.

The location works the same way: map and *Places* views read the GPS stored in the EXIF.

#### How sync and backup tools notice changes

OneDrive, Dropbox, Google Drive, iCloud Drive, rsync and most backup programs don't look at EXIF at all. They decide whether a file has changed mainly from its file **modified date** and **size**; some also compare the content.

- **Update file modified date off:** the photo's content has changed but its modified date hasn't. A tool that relies on the date may decide nothing changed and never upload the tagged photo, so the cloud or backup copy stays untagged. rsync's normal quick check (size + modified date) is a clear case: overwriting a date taken can leave the size unchanged, so rsync skips the file.
- **Update file modified date on:** the modified date becomes the time of tagging, so sync and backup tools see a changed file and upload it.

If a photo was already uploaded by a camera-upload feature (Google Photos backup, OneDrive camera roll), the tagged version may arrive as a **second copy** instead of replacing the first. Check the app after a large run.

#### Which to use

| You want… | Do this |
|---|---|
| Photos in the right place on the timeline | Set a **Date taken** (tick *Overwrite existing date taken* if they already have a wrong one) |
| OneDrive, Dropbox, Google Drive or a backup tool to pick up the tagged photos | Tick **Update file modified date** |
| File managers to keep showing the original dates (folder isn't synced) | Leave **Update file modified date** off |
| Photos with **no** date taken, in a synced folder | Tick **Update file modified date** *and* set a **Date taken**. Otherwise apps that fall back to the file date may move those photos to the day you tagged them. |

**Update file modified date** is off by default, and resets to off each time the app starts, so tick it deliberately for synced folders.

#### What happens to each date

| Date | Update file modified date **on** | **off** |
|---|---|---|
| File *modified* | Time of tagging | Original kept |
| File *accessed* | Time of tagging | Original kept |
| EXIF *Date Taken* and *Digitized* (inside the photo) | Only changed if you set a date taken | Same |
| EXIF *Modified* (DateTime, inside the photo) | Set to the time of tagging whenever the photo's EXIF is changed | Same |
| Folder *modified* | Changes | Changes |
| *Created* / birth time (e.g. Windows "Date created") | Becomes the time of tagging | Becomes the time of tagging |

---

### Setting the date taken

The **Date taken** box sets the date and time a photo was taken, for example for scanned prints or a camera whose clock was never set.

- It writes two EXIF tags: **Date Taken** (DateTimeOriginal) and **Digitized** (CreateDate). The EXIF **Modified** tag (DateTime) is set to the current time whenever the app changes a photo's EXIF, whether that's the date, GPS, or both.
- Pick the date from the calendar, then **type the time** in the box beside it. Accepted: `9:15`, `09:15:30`, `21:15`, `9:15pm`, `12am`, `0915`, `9.15`. Leave it blank for **12:00 noon**. The line underneath shows exactly what will be written, and turns into a warning if the time isn't understood.
- Every photo in the run gets **the same** date and time.
- Photos that already have a date taken keep it, unless **Overwrite existing date taken** is ticked. Kept dates are logged as warnings.
- The time is stored as local time, as cameras do. No time zone is recorded.
- You can set a date only, a location only, or both. A photo is counted in *Apply to N* if it would get at least one of them.

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

## Settings and place search

Place search uses OpenStreetMap's free **Nominatim** service. Click the **⚙** button at the top-right of the left panel to open **Settings**:

| Setting | What it does |
|---|---|
| **Contact email** | Sent with each search so Nominatim's operators can contact you rather than block the app. Optional but recommended. It isn't sent anywhere else. The box warns if the value doesn't look like an email address. |
| **Nominatim server** | Use a different Nominatim server, such as a self-hosted one. **Reset** puts back `https://nominatim.openstreetmap.org`. |
| **Search language** | Language for place names, as codes: `en`, `af`, `mi`, `de`… Several can be listed in order of preference, e.g. `en,mi`. Blank means `en`. |
| **Results per search** | How many matches a search shows, 1–10 (default 5). |
| **Clear cache** | Searches are cached, so repeating one is instant and doesn't contact the server. Clearing the cache makes the next searches fetch fresh results. |
| **Show in folder** | Opens the folder holding `settings.json` in your file manager. |

**Save** checks the values (for example, the server must start with `https://`) and shows a message if something needs fixing. **Cancel** or **Esc** closes without saving. Changes apply to the next search with no restart needed.

The settings are kept in a small file, which you can also edit by hand:

| System | Location |
|---|---|
| Linux | `~/.config/com.zulupilgrim.bulkgeotag/settings.json` |
| Windows | `%APPDATA%\com.zulupilgrim.bulkgeotag\settings.json` |
| macOS | `~/Library/Application Support/com.zulupilgrim.bulkgeotag/settings.json` |

```json
{
  "nominatim_url": "https://nominatim.openstreetmap.org",
  "contact_email": "",
  "language": "en",
  "max_results": 5
}
```

To follow Nominatim's rules, searches happen only when you press Enter or Find, at most one per second. The cache lives in `~/.cache/com.zulupilgrim.bulkgeotag/geocode-cache.json` on Linux (`%LOCALAPPDATA%\com.zulupilgrim.bulkgeotag\` on Windows) and is safe to delete.

---

## What gets remembered

Between launches the app remembers:

- **Bulk folder** or **Single file** mode
- the photo folder (bulk mode), which is rescanned on start
- **Back up originals** and the backup folder
- the log folder
- the map's position and zoom
- whether the photo details card (single-file mode) is folded

**Not** remembered, on purpose: the single photo you last opened, the location or pin (each batch is usually a different place), the *Include subfolders* and *Overwrite* boxes (safer to tick deliberately each time), the date taken box and its overwrite box, and *Update file modified date*, which starts **off** at every launch.

---

## Troubleshooting

| Message / symptom | Cause and fix |
|---|---|
| **Apply stays faded** | It needs a folder, a location and/or a date taken, at least one photo to write, and a backup folder if backup is ticked. A scan might also still be running. |
| **"Nothing to write"** | Neither a location nor a date taken is set. |
| **"The backup folder is inside the photo folder"** | Select a backup folder outside the photo folder. Otherwise backups would be scanned and tagged next time. |
| **Search failed: … 403** | Nominatim refused the request. Check the contact email in **⚙ Settings** for typos, or try again later. |
| **Search failed: … 429** | Too many requests. Wait a minute. |
| **Search failed: could not reach …** | No internet connection, or `nominatim_url` is wrong. |
| **No places found** | Add a country or region (*Milford Sound, New Zealand*), or click the map instead. |
| **Map tiles are grey** | No internet connection. You can still type Lat/Lon. |
| **✕ "unreadable metadata"** | The file's metadata is damaged or it isn't really an image. It was not changed. |
| **✕ "verification failed"** | The write didn't check out (wrong coordinates read back, or camera data would have changed), so the original was kept. Please report it with the file type and camera model. |
| **TIFF ✕ "requires XResolution"** | Some generated TIFFs lack resolution tags. Camera and scanner TIFFs normally have them. |
| **"Log NOT written: …"** | The log folder is missing or can't be written to. |
| **"Drop one folder or one photo at a time"** | Several items were dropped together. Drop just one folder, or just one photo. |
| **"… can't be tagged yet" / "is not a supported photo"** after a drop | Only JPEG, TIFF and WebP photos can be tagged. Drop a folder to tag all its photos at once. |
| **The Browse… window shows no thumbnails** | That window belongs to the operating system; on Linux Mint (GTK) it only has a list view. Use [drag and drop](#drag-and-drop) from your file manager's thumbnail view instead. |
| **Folder chooser button says "Open"** | That's normal on Linux. The label comes from the system's file chooser. Clicking it selects the folder. |

---

## Limitations

- In single-file mode, TIFF photos have no preview (the app's built-in viewer can't show TIFF) unless they contain an embedded thumbnail.
- PNG, HEIC/HEIF (iPhone) and RAW files are not written yet. PNG is skipped and logged. HEIC and RAW are not scanned at all.
- Only latitude/longitude and date taken are written, not altitude, direction or time zone.
- The date taken is the same for every photo in a run; shifting each photo's own date by an offset isn't supported yet.
- There's no undo apart from the backups, so use **Back up originals** for anything you can't afford to lose.
- The development build (`npm run tauri dev`) and the installed app keep separate remembered settings.
