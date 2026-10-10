# Development time

The below shows what can be achieved when one non-coder vibe codes with Claude AI.

How long Bulk GeoTag took to build, from the first message to packaged releases for Linux, Windows and macOS.
All times are NZ local time (NZDT, UTC+13) on **Saturday 10 October 2026**.

*Working time* counts only time spent on the project. The timer was paused during breaks, so *wall clock* (start to finish, breaks included) is longer.

## Summary

| | |
|---|---|
| Started | 13:29 |
| Completed | 23:02 |
| **Working time** | **5h 33m** |
| Wall clock | 9h 33m |
| Breaks | 4h 00m |
| Waiting for package builds (included in working time) | ~38 min |

## Work sessions

| Session | From | To | Duration | Notes |
|---|---|---|---|---|
| 1 | 13:29 | 15:56 | 2h 27m | Setup, UI, scan, place search, writing GPS, backups, log, docs |
| — | 15:56 | 16:03 | *(break)* | |
| 2 | 16:03 | 16:10 | 7m | File-date question; *Update modified date* option started |
| — | 16:10 | 20:03 | *(break)* | |
| 3 | 20:03 | 23:02 | 2h 59m | Date options, date taken, single-file mode, settings, packaging, Windows, macOS |
| **Total** | | | **5h 33m** | |

## Milestones

| Time | Milestone | Git tag | Working time |
|---|---|---|---|
| 20:42 | **MVP**: scan, search, map, date taken, preview/apply, backups, log, docs | `mvp-1.0` | 3h 13m |
| 21:10 | Single-file mode | `v1.1` | 3h 41m |
| 21:17 | Code comments, changelog | `v1.1.1` | 3h 48m |
| 21:25 | ⚙ Settings dialog: **finished app, before packaging** | `v1.2` | 3h 56m |
| 21:58 | Linux packages (.deb, .rpm, AppImage), installed from the Mint menu | `v1.2-packaged` | 4h 29m |
| 22:02 | First GitHub Release with downloadable Linux packages | — | 4h 33m |
| 22:33 | Windows installers built and **confirmed working** on a Windows VM | — | 5h 04m |
| 22:47 | macOS build (universal .dmg) on the release. **Not tested on a Mac: no Mac hardware available.** | — | 5h 18m |
| 23:02 | **Project complete**: Linux, Windows and macOS packages on the GitHub Release; docs finished | — | **5h 33m** |

## Waiting for package builds

Time spent waiting for packages to finish building, included in the working time above.

| Build | Where | Started | Finished | Wait |
|---|---|---|---|---|
| Linux: .deb, .rpm, AppImage | Local (`npm run tauri build`) | 21:44 | 21:54 | ~10 min (4 min compile + bundling + one-off AppImage tool download) |
| Windows: .msi, setup.exe | GitHub Actions (first run, no cache) | 22:08 | 22:25 | 17 min 30 s |
| macOS: universal .dmg | GitHub Actions (first run, no cache) | 22:37 | 22:47 | 10 min 21 s |
| **Total** | | | | **~38 min** |

Shorter compile waits during development (the first `tauri dev` start, the first `cargo test`, and the first builds after adding new Rust libraries, a few minutes each) are not counted here.

## Method

- The start time is the first message of the build session.
- Breaks were logged when the developer said *pause* and *resume*. Gaps between messages within a session count as working time.
- Milestone times are when each version was committed and tagged, or confirmed working.
- Build times come from the terminal output (local) and from the GitHub Actions run timestamps.
