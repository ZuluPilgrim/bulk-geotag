<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open, ask } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import "leaflet/dist/leaflet.css";
  import type { Map as LMap, Marker } from "leaflet";

  type Candidate = { label: string; lat: number; lon: number };

  // ---------- Photos ----------
  // ---------- Remembered settings (this computer only, browser storage of the app) ----------
  type Saved = {
    folder?: string;
    backupEnabled?: boolean;
    backupFolder?: string;
    logFolder?: string;
    mapCenter?: [number, number];
    mapZoom?: number;
  };
  const STORE_KEY = "bulk-geotag.settings.v1";
  function loadSaved(): Saved {
    try {
      return JSON.parse(localStorage.getItem(STORE_KEY) ?? "{}");
    } catch {
      return {};
    }
  }
  function save(patch: Saved) {
    try {
      localStorage.setItem(STORE_KEY, JSON.stringify({ ...loadSaved(), ...patch }));
    } catch {
      /* storage unavailable — settings just won't be remembered */
    }
  }
  const saved = loadSaved();

  let folder = $state(saved.folder ?? "");
  let includeSubfolders = $state(false);
  let overwriteGps = $state(false);

  type PhotoInfo = { path: string; gps: "present" | "absent" | "unreadable"; issue: string | null };
  type ScanResult = {
    photos: number;
    with_gps: number;
    unreadable: number;
    files: PhotoInfo[];
    folder_errors: PhotoInfo[];
  };

  let scan = $state<ScanResult | null>(null);
  let scanning = $state(false);
  let scanSeq = 0; // ignores results from an older scan if settings change mid-scan

  // Writable today: JPEG, TIFF, WebP (PNG later). Unreadable files never; files with GPS only when overwriting.
  const WRITABLE = /\.(jpe?g|tiff?|webp)$/i;
  let toWrite = $derived(
    scan
      ? scan.files.filter(
          (f) => WRITABLE.test(f.path) && f.gps !== "unreadable" && (overwriteGps || f.gps !== "present")
        ).length
      : 0
  );
  let skipped = $derived(scan ? scan.photos - toWrite : 0);

  async function runScan(path: string, recursive: boolean) {
    const seq = ++scanSeq;
    if (!path) {
      scan = null;
      return;
    }
    scanning = true;
    status = "Scanning…";
    try {
      const result = await invoke<ScanResult>("scan_folder", { path, recursive });
      if (seq !== scanSeq) return;
      scan = result;
      const issues = result.unreadable + result.folder_errors.length;
      status = `Scan complete: ${result.photos} photos${issues ? ` · ${issues} with problems` : ""}.`;
    } catch (e) {
      if (seq !== scanSeq) return;
      scan = null;
      status = `Scan failed: ${e}`;
    } finally {
      if (seq === scanSeq) scanning = false;
    }
  }

  // Rescan whenever the folder or the subfolder option changes.
  $effect(() => {
    const path = folder;
    const recursive = includeSubfolders;
    untrack(() => runScan(path, recursive));
  });

  // ---------- Location ----------
  let query = $state("");
  let searching = $state(false);
  let candidates = $state<Candidate[]>([]);
  let selected = $state<number | null>(null);
  let lat = $state<number | null>(null);
  let lon = $state<number | null>(null);

  // ---------- Backup ----------
  let backupEnabled = $state(saved.backupEnabled ?? false);
  let backupFolder = $state(saved.backupFolder ?? "");

  // ---------- Log ----------
  let logFolder = $state(saved.logFolder ?? "");

  // Save whenever any remembered setting changes.
  $effect(() => {
    save({ folder, backupEnabled, backupFolder, logFolder });
  });

  // ---------- Status ----------
  let status = $state("Select a photo folder to begin.");
  let busy = $state(false); // Preview/Apply running

  let canApply = $derived(
    !!folder && !scanning && !busy && lat !== null && lon !== null && toWrite > 0 && (!backupEnabled || !!backupFolder)
  );

  // ---------- Map ----------
  let mapEl: HTMLElement;
  let L: typeof import("leaflet");
  let map: LMap | undefined;
  let candidateMarkers: Marker[] = [];
  let pin: Marker | null = null;

  function numberedIcon(n: number, active: boolean) {
    return L.divIcon({
      className: "",
      html: `<div class="cand ${active ? "active" : ""}">${n}</div>`,
      iconSize: [28, 28],
      iconAnchor: [14, 14],
    });
  }

  function pinIcon() {
    return L.divIcon({ className: "", html: `<div class="pin"></div>`, iconSize: [22, 22], iconAnchor: [11, 26] });
  }

  onMount(() => {
    let disposed = false;
    (async () => {
      L = await import("leaflet");
      if (disposed) return;
      map = L.map(mapEl).setView(saved.mapCenter ?? [20, 0], saved.mapZoom ?? 2);
      map.on("moveend", () => {
        const c = map!.getCenter();
        save({ mapCenter: [c.lat, c.lng], mapZoom: map!.getZoom() });
      });
      L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
        maxZoom: 19,
        attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
      }).addTo(map);
      map.on("click", (e) => setPin(e.latlng.lat, e.latlng.lng, true));
    })();
    return () => {
      disposed = true;
      map?.remove();
    };
  });

  function round(n: number) {
    return Math.round(n * 1e6) / 1e6;
  }

  function setPin(newLat: number, newLon: number, manual = false) {
    lat = round(newLat);
    lon = round(newLon);
    if (!map) return;
    if (!pin) {
      pin = L.marker([newLat, newLon], { draggable: true, icon: pinIcon(), zIndexOffset: 1000 }).addTo(map);
      pin.on("dragend", () => {
        const p = pin!.getLatLng();
        lat = round(p.lat);
        lon = round(p.lng);
        markManual("Pin moved manually.");
      });
    } else {
      pin.setLatLng([newLat, newLon]);
    }
    if (manual) markManual("Pin placed manually.");
  }

  function markManual(msg: string) {
    selected = null;
    drawCandidates();
    status = msg;
  }

  function onCoordInput() {
    if (lat === null || lon === null || isNaN(lat) || isNaN(lon)) return;
    if (lat < -90 || lat > 90 || lon < -180 || lon > 180) {
      status = "Latitude must be −90…90 and longitude −180…180.";
      return;
    }
    setPin(lat, lon, true);
    map?.panTo([lat, lon]);
  }

  function drawCandidates() {
    if (!map) return;
    candidateMarkers.forEach((m) => m.remove());
    candidateMarkers = candidates.map((c, i) =>
      L.marker([c.lat, c.lon], { icon: numberedIcon(i + 1, i === selected) })
        .addTo(map!)
        .on("click", () => choose(i))
    );
  }

  function choose(i: number) {
    selected = i;
    const c = candidates[i];
    setPin(c.lat, c.lon);
    map?.setView([c.lat, c.lon], 14);
    drawCandidates();
    status = `Selected: ${c.label}`;
  }

  // ---------- Actions ----------
  async function browsePhotos() {
    const picked = await open({ directory: true, multiple: false, defaultPath: folder || undefined, title: "Select photo folder" });
    if (typeof picked === "string") {
      folder = picked;
    }
  }

  async function browseLog() {
    const picked = await open({ directory: true, multiple: false, defaultPath: logFolder || undefined, title: "Select log folder" });
    if (typeof picked === "string") logFolder = picked;
  }

  async function browseBackup() {
    const picked = await open({ directory: true, multiple: false, defaultPath: backupFolder || undefined, title: "Select backup folder" });
    if (typeof picked === "string") backupFolder = picked;
  }

  async function find(e: Event) {
    e.preventDefault();
    const q = query.trim();
    if (!q || !map || searching) return;
    searching = true;
    status = `Searching for "${q}"…`;
    try {
      candidates = await invoke<Candidate[]>("geocode", { query: q });
    } catch (err) {
      candidates = [];
      status = `Search failed: ${err}`;
      return;
    } finally {
      searching = false;
      selected = null;
      drawCandidates();
    }
    if (candidates.length === 0) {
      status = `No places found for "${q}". Try adding a country or region, or set the pin by clicking the map.`;
      return;
    }
    if (candidates.length === 1) {
      choose(0);
    } else {
      map.fitBounds(L.latLngBounds(candidates.map((c) => [c.lat, c.lon] as [number, number])), {
        padding: [40, 40],
        maxZoom: 14,
      });
      status = `${candidates.length} possible locations — pick one or set the pin yourself.`;
    }
  }

  // ---------- Preview / Apply ----------
  type Entry = { time: string; level: "info" | "warning" | "error"; path: string; message: string };
  type ApplyResult = {
    dry_run: boolean;
    total: number;
    written: number;
    skipped: number;
    warnings: number;
    errors: number;
    entries: Entry[];
    log_file: string | null;
    log_error: string | null;
  };

  let progress = $state<[number, number] | null>(null);
  let results = $state<ApplyResult | null>(null);
  let showInfo = $state(true);

  function request(dryRun: boolean) {
    return {
      root: folder,
      recursive: includeSubfolders,
      overwrite: overwriteGps,
      lat: lat!,
      lon: lon!,
      backupDir: backupEnabled && backupFolder ? backupFolder : null,
      logDir: logFolder || null,
      dryRun,
    };
  }

  async function runApply(dryRun: boolean) {
    busy = true;
    progress = null;
    status = dryRun ? "Preparing preview…" : "Writing GPS…";
    const unlisten = await listen<[number, number]>("apply-progress", (e) => (progress = e.payload));
    let summary = "";
    try {
      const r = await invoke<ApplyResult>("apply_gps", { request: request(dryRun) });
      results = r;
      showInfo = true;
      const head = dryRun ? `Preview: ${r.written} would be written` : `Done: ${r.written} written`;
      let msg = `${head} · ${r.skipped} skipped · ${r.warnings} warnings · ${r.errors} errors`;
      if (dryRun) msg += " — nothing changed.";
      if (r.log_file) msg += ` · Log written: ${r.log_file}`;
      if (r.log_error) msg += ` · Log NOT written: ${r.log_error}`;
      status = summary = msg;
    } catch (err) {
      status = `${dryRun ? "Preview" : "Apply"} failed: ${err}`;
    } finally {
      unlisten();
      busy = false;
      progress = null;
    }
    if (!dryRun) {
      await runScan(folder, includeSubfolders);
      if (summary) status = summary; // keep the Apply summary (and log name) visible after the rescan
    }
  }

  function preview() {
    runApply(true);
  }

  async function apply() {
    const lines = [
      `Write GPS ${lat}, ${lon} to ${toWrite} photo${toWrite === 1 ? "" : "s"}?`,
      "",
      `Folder: ${folder}${includeSubfolders ? " (and subfolders)" : ""}`,
      `Overwrite existing GPS: ${overwriteGps ? "yes" : "no"}`,
      `Backup originals: ${backupEnabled ? backupFolder : "no"}`,
      `Log: ${logFolder || "none"}`,
    ];
    const ok = await ask(lines.join("\n"), { title: "Confirm Apply", kind: "warning", okLabel: "Write GPS", cancelLabel: "Cancel" });
    if (ok) runApply(false);
  }

  function fileName(path: string) {
    return path.split(/[\\/]/).pop() ?? path;
  }
</script>

<div class="app">
  <aside class="panel">
    <section>
      <h2>Photos</h2>
      <label class="field-label" for="folder">Folder</label>
      <div class="row">
        <input id="folder" type="text" value={folder} placeholder="Select a folder…" readonly />
        <button type="button" class="primary" onclick={browsePhotos}>Browse…</button>
      </div>
      <label class="check"><input type="checkbox" bind:checked={includeSubfolders} /> Include subfolders</label>
      <label class="check"><input type="checkbox" bind:checked={overwriteGps} /> Overwrite existing GPS</label>
      {#if scanning}
        <p class="meta">Scanning…</p>
      {:else if scan}
        <p class="meta">
          {scan.photos} photos · {scan.with_gps} already have GPS
          {#if scan.unreadable}· <span class="warn">{scan.unreadable} unreadable</span>{/if}
          {#if scan.folder_errors.length}· <span class="warn">{scan.folder_errors.length} folders skipped</span>{/if}
        </p>
      {/if}
    </section>

    <section>
      <h2>Location</h2>
      <form class="row" onsubmit={find}>
        <input type="text" bind:value={query} placeholder="e.g. Colosseum, Rome" />
        <button type="submit" disabled={searching || !query.trim()}>{searching ? "…" : "Find"}</button>
      </form>

      {#if candidates.length}
        <ul class="candidates">
          {#each candidates as c, i}
            <li>
              <label class:active={selected === i}>
                <input type="radio" name="cand" checked={selected === i} onchange={() => choose(i)} />
                <span class="num">{i + 1}</span>
                <span>{c.label}</span>
              </label>
            </li>
          {/each}
        </ul>
      {/if}
      <p class="hint">Or click the map / drag the red pin to set the exact spot.</p>

      <div class="coords">
        <label>Lat <input type="number" step="0.000001" bind:value={lat} onchange={onCoordInput} /></label>
        <label>Lon <input type="number" step="0.000001" bind:value={lon} onchange={onCoordInput} /></label>
      </div>
    </section>

    <section>
      <h2>Backup</h2>
      <label class="check"><input type="checkbox" bind:checked={backupEnabled} /> Back up originals</label>
      <label class="field-label" for="backup">Backup to</label>
      <div class="row">
        <input
          id="backup"
          type="text"
          value={backupFolder}
          placeholder="Select a backup folder…"
          readonly
          disabled={!backupEnabled}
        />
        <button type="button" class="primary" onclick={browseBackup} disabled={!backupEnabled}>Browse…</button>
      </div>
    </section>

    <section>
      <h2>Log</h2>
      <label class="field-label" for="logdir">Write errors &amp; warnings to</label>
      <div class="row">
        <input id="logdir" type="text" value={logFolder} placeholder="No log (optional)" readonly />
        {#if logFolder}
          <button type="button" onclick={() => (logFolder = "")} title="Stop logging">✕</button>
        {/if}
        <button type="button" class="primary" onclick={browseLog}>Browse…</button>
      </div>
    </section>

    <section class="actions">
      <button type="button" class="go" onclick={preview} disabled={!canApply}>Preview</button>
      <button type="button" class="primary" onclick={apply} disabled={!canApply}>Apply to {toWrite}</button>
    </section>
  </aside>

  <div class="mapwrap">
    <main class="map" bind:this={mapEl}></main>

    {#if results}
      <div class="results">
        <header>
          <strong>{results.dry_run ? "Preview" : "Results"}</strong>
          <span>
            {results.written} {results.dry_run ? "to write" : "written"} · {results.skipped} skipped ·
            {results.warnings} warnings · {results.errors} errors
          </span>
          <label class="check small"><input type="checkbox" bind:checked={showInfo} /> Show OK files</label>
          <button type="button" onclick={() => (results = null)}>Close</button>
        </header>
        <ul>
          {#each results.entries.filter((e) => showInfo || e.level !== "info") as e}
            <li class={e.level}>
              <span class="lvl">{e.level === "info" ? "✓" : e.level === "warning" ? "!" : "✕"}</span>
              <span class="file" title={e.path}>{fileName(e.path)}</span>
              <span class="msg">{e.message}</span>
            </li>
          {:else}
            <li class="info"><span class="msg">Nothing to show.</span></li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>

  <footer class="status">
    {#if progress}
      <progress max={progress[1] || 1} value={progress[0]}></progress>
      <span>{progress[0]} / {progress[1]}</span>
    {/if}
    <span class="status-text">{status}</span>
  </footer>
</div>

<style>
  :global(:root) {
    --bg: #f6f7f9;
    --panel: #ffffff;
    --text: #1d2330;
    --muted: #6b7385;
    --border: #dfe3ea;
    --accent: #2563eb;
    --accent-text: #ffffff;
    --go: #16a34a;
    --input: #ffffff;
    font-family: system-ui, -apple-system, "Segoe UI", Ubuntu, sans-serif;
    font-size: 14px;
    color: var(--text);
    background: var(--bg);
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #14171d;
      --panel: #1c2028;
      --text: #e6e8ee;
      --muted: #9097a6;
      --border: #2e3440;
      --accent: #4f8cff;
      --go: #2fb565;
      --input: #12151b;
    }
  }
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }

  .app {
    display: grid;
    grid-template-columns: 340px 1fr;
    grid-template-rows: 1fr auto;
    height: 100vh;
  }
  .panel {
    background: var(--panel);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    padding: 4px 16px 16px;
  }
  section {
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  section:last-child {
    border-bottom: none;
  }
  h2 {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
    margin: 0 0 8px;
  }
  .field-label {
    display: block;
    font-size: 12px;
    color: var(--muted);
    margin: 6px 0 4px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  input[type="text"],
  input[type="number"] {
    background: var(--input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
    font: inherit;
  }
  input:disabled {
    opacity: 0.5;
  }
  button {
    font: inherit;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--panel);
    color: var(--text);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button.primary {
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  button.go {
    background: var(--go);
    color: #ffffff;
    border-color: var(--go);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }
  .meta,
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 8px 0 0;
  }
  .warn {
    color: #d97706;
    font-weight: 600;
  }
  .candidates {
    list-style: none;
    padding: 0;
    margin: 10px 0 0;
  }
  .candidates label {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px;
    border-radius: 6px;
    cursor: pointer;
  }
  .candidates label.active {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .num {
    flex: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--muted);
    color: #fff;
    font-size: 11px;
    display: grid;
    place-items: center;
  }
  .active .num {
    background: var(--accent);
  }
  .coords {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
  .coords label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--muted);
    flex: 1;
  }
  .coords input {
    width: 100%;
    min-width: 0;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .actions button {
    flex: 1;
  }
  .mapwrap {
    position: relative;
    min-height: 0;
  }
  .map {
    position: absolute;
    inset: 0;
  }
  .results {
    position: absolute;
    inset: 16px;
    z-index: 1000; /* above Leaflet panes */
    display: flex;
    flex-direction: column;
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25);
    overflow: hidden;
  }
  .results header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .results header span {
    color: var(--muted);
    font-size: 12px;
    flex: 1;
  }
  .check.small {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .results ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
  }
  .results li {
    display: grid;
    grid-template-columns: 22px minmax(120px, 30%) 1fr;
    gap: 8px;
    padding: 5px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .results .lvl {
    font-weight: 700;
    text-align: center;
  }
  .results .info .lvl {
    color: var(--go);
  }
  .results .warning .lvl {
    color: #d97706;
  }
  .results .error .lvl,
  .results .error .msg {
    color: #dc2626;
  }
  .results .file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    grid-column: 1 / -1;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--muted);
    background: var(--panel);
    border-top: 1px solid var(--border);
  }

  /* Leaflet markers are divIcons, so there are no image paths to break */
  :global(.cand) {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #6b7385;
    color: #fff;
    font: 600 13px system-ui, sans-serif;
    display: grid;
    place-items: center;
    border: 2px solid #fff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    cursor: pointer;
  }
  :global(.cand.active) {
    background: #2563eb;
  }
  :global(.pin) {
    width: 22px;
    height: 22px;
    background: #e11d48;
    border: 2px solid #fff;
    border-radius: 50% 50% 50% 0;
    transform: rotate(-45deg);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
    cursor: grab;
  }
</style>
