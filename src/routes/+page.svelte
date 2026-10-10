<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import "leaflet/dist/leaflet.css";
  import type { Map as LMap, Marker } from "leaflet";

  type Candidate = { label: string; lat: number; lon: number };

  // ---------- Photos ----------
  let folder = $state("");
  let includeSubfolders = $state(false);
  let overwriteGps = $state(false);

  // FAKE scan results — replaced by the Rust folder scan in a later step
  const fakeScan = {
    top: { photos: 40, withGps: 6 },
    recursive: { photos: 128, withGps: 14 },
  };
  let scan = $derived(folder ? (includeSubfolders ? fakeScan.recursive : fakeScan.top) : null);
  let toWrite = $derived(scan ? (overwriteGps ? scan.photos : scan.photos - scan.withGps) : 0);
  let skipped = $derived(scan ? scan.photos - toWrite : 0);

  // ---------- Location ----------
  let query = $state("");
  let searching = $state(false);
  let candidates = $state<Candidate[]>([]);
  let selected = $state<number | null>(null);
  let lat = $state<number | null>(null);
  let lon = $state<number | null>(null);

  // ---------- Backup ----------
  let backupEnabled = $state(false);
  let backupFolder = $state("");

  // ---------- Log ----------
  let logFolder = $state("");

  // ---------- Status ----------
  let status = $state("Choose a photo folder to begin.");

  let canApply = $derived(
    !!folder && lat !== null && lon !== null && toWrite > 0 && (!backupEnabled || !!backupFolder)
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
      map = L.map(mapEl).setView([20, 0], 2);
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

  // ---------- FAKE actions — wired to Rust / Nominatim in later steps ----------
  async function browsePhotos() {
    const picked = await open({ directory: true, multiple: false, defaultPath: folder || undefined, title: "Choose photo folder" });
    if (typeof picked === "string") {
      folder = picked;
      status = "Folder chosen (photo counts are still fake).";
    }
  }

  async function browseLog() {
    const picked = await open({ directory: true, multiple: false, defaultPath: logFolder || undefined, title: "Choose log folder" });
    if (typeof picked === "string") logFolder = picked;
  }

  async function browseBackup() {
    const picked = await open({ directory: true, multiple: false, defaultPath: backupFolder || undefined, title: "Choose backup folder" });
    if (typeof picked === "string") backupFolder = picked;
  }

  async function find(e: Event) {
    e.preventDefault();
    if (!query.trim() || !map) return;
    searching = true;
    status = `Searching for "${query}"…`;
    await new Promise((r) => setTimeout(r, 400));
    candidates = [
      { label: "Colosseum, Rome, Lazio, Italy", lat: 41.890251, lon: 12.492373 },
      { label: "Rome, Floyd County, Georgia, USA", lat: 34.257038, lon: -85.164673 },
      { label: "Rome, Oneida County, New York, USA", lat: 43.212847, lon: -75.455903 },
    ];
    searching = false;
    selected = null;
    drawCandidates();
    map.fitBounds(L.latLngBounds(candidates.map((c) => [c.lat, c.lon] as [number, number])), {
      padding: [40, 40],
    });
    status = `${candidates.length} possible locations (fake data) — pick one or set the pin yourself.`;
  }

  function preview() {
    status = `Preview: ${toWrite} to write · ${skipped} skipped${
      backupEnabled ? " · originals backed up" : ""
    } — dry run, nothing changed.`;
  }

  function apply() {
    status = "Apply clicked — file writing is not built yet.";
  }
</script>

<div class="app">
  <aside class="panel">
    <section>
      <h2>Photos</h2>
      <label class="field-label" for="folder">Folder</label>
      <div class="row">
        <input id="folder" type="text" value={folder} placeholder="Choose a folder…" readonly />
        <button type="button" class="primary" onclick={browsePhotos}>Browse…</button>
      </div>
      <label class="check"><input type="checkbox" bind:checked={includeSubfolders} /> Include subfolders</label>
      <label class="check"><input type="checkbox" bind:checked={overwriteGps} /> Overwrite existing GPS</label>
      {#if scan}
        <p class="meta">{scan.photos} photos · {scan.withGps} already have GPS</p>
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
          placeholder="Choose a backup folder…"
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

  <main class="map" bind:this={mapEl}></main>

  <footer class="status">{status}</footer>
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
  .map {
    min-height: 0;
  }
  .status {
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
