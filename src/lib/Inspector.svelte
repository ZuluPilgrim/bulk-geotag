<script lang="ts">
  // The Inspector: browse folders and photos inside the app. Read-only; the main screen
  // takes the result (folder on screen, last photo selected) when it closes.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { homeDir } from "@tauri-apps/api/path";
  import type { Map as LMap, Marker } from "leaflet";
  import ExifDetails from "$lib/ExifDetails.svelte";
  import type { PhotoDetails } from "$lib/types";

  type Item = { name: string; path: string; linked: boolean };
  type Thumb = { data: string; orientation: number; embedded: boolean };
  type Listing = { path: string; parent: string | null; folders: Item[]; photos: Item[] };

  let {
    start,
    initialPhoto = null,
    onclose,
  }: {
    /** Folder to open (or a photo, whose folder is opened). Empty: the home folder. */
    start: string;
    /** Photo to select at first (single-file mode). */
    initialPhoto?: string | null;
    /** Called by the ✕ button; the parent then reads result() and closes. */
    onclose: () => void;
  } = $props();

  let listing = $state<Listing | null>(null);
  let error = $state("");
  let loading = $state(false);
  // The last photo clicked, in any folder visited.
  let selected = $state<string | null>(null);
  // Only a photo you clicked or moved to with the keys is handed back (not one selected automatically).
  let picked = false;
  let gridEl = $state<HTMLElement | null>(null);

  // ---------- Tile size (remembered on this computer) ----------
  const SIZE_KEY = "bulk-geotag.inspector.v1";
  const MIN_TILE = 100;
  const MAX_TILE = 600;
  const DEFAULT_TILE = 240;
  /** From this tile size up, tiles get the larger (768 px) thumbnails instead of the small ones. */
  const LARGE_FROM = 200;
  function loadSize() {
    try {
      const n = Number(JSON.parse(localStorage.getItem(SIZE_KEY) ?? "{}").tile);
      return n >= MIN_TILE && n <= MAX_TILE ? n : DEFAULT_TILE;
    } catch {
      return DEFAULT_TILE;
    }
  }
  let tile = $state(loadSize());
  let large = $derived(tile >= LARGE_FROM);
  $effect(() => {
    try {
      localStorage.setItem(SIZE_KEY, JSON.stringify({ tile }));
    } catch {
      /* not remembered */
    }
  });

  // ---------- Thumbnails: only for tiles near the visible area, 4 at a time ----------
  // "S|path" / "L|path" → thumbnail, or null when the photo can't be shown. Kept while open.
  let thumbs = $state<Record<string, Thumb | null>>({});
  const key = (path: string, big: boolean) => `${big ? "L" : "S"}|${path}`;
  /** The best thumbnail there is for a tile: the right size, else the other size while it loads. */
  function thumbFor(path: string): Thumb | null | undefined {
    const want = thumbs[key(path, large)];
    return want !== undefined ? want : thumbs[key(path, !large)];
  }
  const queue: string[] = []; // keys
  let running = 0;
  const MAX_AT_ONCE = 4;
  const visible = new Set<string>(); // photo paths near the visible area

  function want(path: string) {
    const k = key(path, large);
    if (k in thumbs || queue.includes(k)) return;
    queue.push(k);
    pump();
  }

  function pump() {
    while (running < MAX_AT_ONCE && queue.length) {
      const k = queue.shift()!;
      const path = k.slice(2);
      running++;
      invoke<Thumb | null>("photo_thumbnail", { path, large: k.startsWith("L") })
        .then((t) => (thumbs[k] = t))
        .catch(() => (thumbs[k] = null))
        .finally(() => {
          running--;
          pump();
        });
    }
  }

  // Crossing the large/small line: fetch the other size for the tiles on screen.
  $effect(() => {
    void large;
    for (const p of visible) want(p);
  });

  let observer: IntersectionObserver | null = null;
  /** Svelte action on each photo tile: track whether it's near the visible area, and ask for its thumbnail then. */
  function lazy(node: HTMLElement, path: string) {
    observer ??= new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          const p = (e.target as HTMLElement).dataset.path!;
          if (e.isIntersecting) {
            visible.add(p);
            want(p);
          } else visible.delete(p);
        }
      },
      { root: gridEl, rootMargin: "300px" }
    );
    node.dataset.path = path;
    observer.observe(node);
    return {
      destroy: () => {
        observer?.unobserve(node);
        visible.delete(path);
      },
    };
  }

  // ---------- Right pane: mini map and the selected photo's details ----------
  let mapEl = $state<HTMLElement | null>(null);
  let L: typeof import("leaflet") | undefined;
  let map: LMap | undefined;
  let marker: Marker | null = null;
  let details = $state<PhotoDetails | null>(null);
  let detailsError = $state("");
  let detailsSeq = 0; // ignore answers for a photo that's no longer selected

  onMount(() => {
    let disposed = false;
    (async () => {
      L = await import("leaflet");
      if (disposed || !mapEl) return;
      map = L.map(mapEl, { zoomControl: true, attributionControl: true }).setView([20, 0], 1);
      L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
        maxZoom: 19,
        attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
      }).addTo(map);
      showOnMap(details);
    })();
    return () => {
      disposed = true;
      map?.remove();
    };
  });

  /** Pin the photo's GPS on the mini map, or show the whole world when it has none. */
  function showOnMap(d: PhotoDetails | null) {
    if (!map || !L) return;
    marker?.remove();
    marker = null;
    if (d && d.lat != null && d.lon != null) {
      marker = L.marker([d.lat, d.lon], {
        icon: L.divIcon({ className: "", html: `<div class="pin"></div>`, iconSize: [22, 22], iconAnchor: [11, 26] }),
        interactive: false,
      }).addTo(map);
      map.setView([d.lat, d.lon], 13);
    } else {
      map.setView([20, 0], 1);
    }
  }

  // Read the selected photo's details whenever the selection changes.
  $effect(() => {
    const path = selected;
    const seq = ++detailsSeq;
    if (!path) {
      details = null;
      showOnMap(null);
      return;
    }
    invoke<PhotoDetails>("inspect_photo", { path })
      .then((d) => {
        if (seq !== detailsSeq) return;
        details = d;
        detailsError = "";
        showOnMap(d);
      })
      .catch((e) => {
        if (seq !== detailsSeq) return;
        details = null;
        detailsError = String(e);
        showOnMap(null);
      });
  });

  // ---------- Keyboard: a cursor over Up, the folders and the photos ----------
  type Entry = { kind: "up" | "folder" | "photo"; path: string };
  let entries = $derived.by((): Entry[] => {
    if (!listing) return [];
    return [
      ...(listing.parent ? [{ kind: "up" as const, path: listing.parent }] : []),
      ...listing.folders.map((f) => ({ kind: "folder" as const, path: f.path })),
      ...listing.photos.map((p) => ({ kind: "photo" as const, path: p.path })),
    ];
  });
  // Key of the entry with the keyboard cursor: "up", or a folder/photo path.
  let cursor = $state<string | null>(null);
  let preview = $state(false); // big preview of the selected photo

  const entryKey = (e: Entry) => (e.kind === "up" ? "up" : e.path);

  /** After opening a folder: the folder we came up from, else the selected photo, else the first photo. */
  function placeCursor(cameFrom?: string) {
    const l = listing!;
    if (cameFrom && l.folders.some((f) => f.path === cameFrom)) cursor = cameFrom;
    else if (selected && l.photos.some((p) => p.path === selected)) cursor = selected;
    else cursor = l.photos[0]?.path ?? (l.parent ? "up" : (l.folders[0]?.path ?? null));
    if (cursor && l.photos.some((p) => p.path === cursor)) selected = cursor;
  }

  /** Move the cursor to an entry; a photo also becomes the selected photo. Keeps it in view. */
  function moveTo(e: Entry | undefined) {
    if (!e) return;
    cursor = entryKey(e);
    if (e.kind === "photo") {
      selected = e.path;
      picked = true;
    }
    gridEl?.querySelector(`[data-key="${CSS.escape(cursor)}"]`)?.scrollIntoView({ block: "nearest" });
  }

  /**
   * The entry one row above or below the cursor, as laid out on screen: the nearest row in that
   * direction, then the item in it closest left-to-right. Works across the folder buttons (which
   * wrap onto several rows) and the photo grid alike.
   */
  function rowNeighbour(dir: 1 | -1): Entry | undefined {
    if (!gridEl || !cursor) return undefined;
    const els = [...gridEl.querySelectorAll<HTMLElement>("[data-key]")];
    const here = els.find((el) => el.dataset.key === cursor);
    if (!here) return undefined;
    const r = here.getBoundingClientRect();
    const mid = r.left + r.width / 2;
    const rows = els
      .map((el) => ({ el, b: el.getBoundingClientRect() }))
      .filter(({ b }) => (dir > 0 ? b.top >= r.bottom - 2 : b.bottom <= r.top + 2));
    if (!rows.length) return undefined;
    // The nearest row in that direction…
    const rowTop = dir > 0 ? Math.min(...rows.map(({ b }) => b.top)) : Math.max(...rows.map(({ b }) => b.top));
    const row = rows.filter(({ b }) => Math.abs(b.top - rowTop) < 4);
    // …and the item in it closest to the cursor's horizontal position.
    row.sort((a, c) => Math.abs(a.b.left + a.b.width / 2 - mid) - Math.abs(c.b.left + c.b.width / 2 - mid));
    const key = row[0].el.dataset.key;
    return entries.find((e) => entryKey(e) === key);
  }

  function activate(e: Entry) {
    if (e.kind === "photo") onclose(); // Enter on a photo: take it and close
    else open(e.path);
  }

  /**
   * Keys while the Inspector is open (called by the main screen before its own handling).
   * Returns true when the key was used here; false lets the main screen handle it (e.g. Esc closes).
   */
  export function handleKey(e: KeyboardEvent): boolean {
    if (preview) {
      if (e.key === "Escape" || e.key === " ") {
        preview = false;
      } else if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
        const photos = entries.filter((x) => x.kind === "photo");
        const i = photos.findIndex((x) => x.path === selected);
        moveTo(photos[i + (e.key === "ArrowRight" ? 1 : -1)]);
      } else return false;
      e.preventDefault();
      return true;
    }
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return false; // e.g. the size slider uses the arrows
    const i = entries.findIndex((x) => entryKey(x) === cursor);
    const cur = entries[i];
    switch (e.key) {
      case "ArrowRight":
        moveTo(entries[i < 0 ? 0 : i + 1]);
        break;
      case "ArrowLeft":
        moveTo(entries[i - 1]);
        break;
      case "ArrowDown":
        moveTo(cur ? rowNeighbour(1) : entries[0]);
        break;
      case "ArrowUp":
        moveTo(rowNeighbour(-1));
        break;
      case "Enter":
        if (cur) activate(cur);
        break;
      case "Backspace":
        if (listing?.parent) open(listing.parent);
        break;
      case " ":
        if (selected) preview = true;
        break;
      default:
        return false;
    }
    e.preventDefault();
    return true;
  }

  /** CSS rotation from the EXIF orientation (1 upright, 3 upside down, 6/8 on their side). */
  function turn(o: number) {
    return ({ 3: "rotate(180deg)", 6: "rotate(90deg)", 8: "rotate(-90deg)" } as Record<number, string>)[o] ?? "";
  }
  function ext(path: string) {
    return (path.split(".").pop() ?? "").toUpperCase();
  }

  /** What closing hands back: the folder on screen and the last photo selected. */
  export function result(): { folder: string | null; photo: string | null; gps: [number, number] | null } {
    const d = picked && details && details.path === selected ? details : null;
    return {
      folder: listing?.path ?? null,
      photo: picked ? selected : null,
      // Where the picked photo is tagged, so the main map can go there.
      gps: d && d.lat != null && d.lon != null ? [d.lat, d.lon] : null,
    };
  }

  /** Show one folder. Errors keep the current folder on screen. */
  async function open(path: string) {
    loading = true;
    const cameFrom = listing?.path;
    try {
      listing = await invoke<Listing>("list_folder", { path });
      placeCursor(cameFrom);
      queue.length = 0; // thumbnails still waiting for the old folder aren't needed
      visible.clear();
      error = "";
      gridEl?.scrollTo(0, 0);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Start where the main screen is (falling back to the home folder if that's gone).
  onMount(async () => {
    selected = initialPhoto;
    const home = await homeDir().catch(() => "/");
    await open(start || home);
    if (!listing) await open(home);
  });

  /** The path as clickable parts: "/home/fred/Pictures" → /, home, fred, Pictures. */
  let crumbs = $derived.by(() => {
    const p = listing?.path ?? "";
    const parts = p.split(/[\\/]+/).filter(Boolean);
    const out: { label: string; path: string }[] = [];
    if (/^[A-Za-z]:/.test(p)) {
      // Windows: C:\ › Users › fred
      let acc = "";
      for (const [i, part] of parts.entries()) {
        acc = i === 0 ? `${part}\\` : `${acc}${i === 1 ? "" : "\\"}${part}`;
        out.push({ label: part, path: acc });
      }
    } else if (p) {
      // Linux/macOS: / › home › fred
      out.push({ label: "/", path: "/" });
      let acc = "";
      for (const part of parts) {
        acc += `/${part}`;
        out.push({ label: part, path: acc });
      }
    }
    return out;
  });

  function photoName(path: string | null) {
    return path ? (path.split(/[\\/]/).pop() ?? path) : "";
  }
</script>

<div class="inspector" role="dialog" aria-label="Inspector">
  <header>
    <button type="button" class="title on" onclick={onclose} title="Close the Inspector (🖼, ✕ or Esc)" aria-pressed="true">🖼 Inspector</button>
    <nav class="crumbs" aria-label="Folder">
      {#each crumbs as c, i (c.path)}
        {#if i > 0 && crumbs[i - 1].label !== "/"}<span class="sep">›</span>{/if}
        <button type="button" class="crumb" class:current={i === crumbs.length - 1} onclick={() => open(c.path)} title={c.path}>{c.label}</button>
      {/each}
    </nav>
    <label class="size" title="Tile size">
      <span aria-hidden="true">▫</span>
      <input type="range" min={MIN_TILE} max={MAX_TILE} step="20" bind:value={tile} aria-label="Tile size" />
      <span aria-hidden="true">◻</span>
    </label>
    <button type="button" class="icon close" onclick={onclose} title="Close (Esc)" aria-label="Close">✕</button>
  </header>

  <div class="body">
    <div class="grid" bind:this={gridEl}>
      {#if error}<p class="error">{error}</p>{/if}
      {#if listing}
        {#if listing.parent || listing.folders.length}
          <div class="folders">
            {#if listing.parent}
              <button
                type="button"
                class="folder up"
                class:cursor={cursor === "up"}
                data-key="up"
                onclick={() => (cursor = "up")}
                ondblclick={() => open(listing!.parent!)}
                title="Up to {listing.parent} (double-click, or Backspace)"
              >
                <span class="glyph">↑</span><span class="name">Up</span>
              </button>
            {/if}
            {#each listing.folders as f (f.path)}
              <button
                type="button"
                class="folder"
                class:cursor={cursor === f.path}
                data-key={f.path}
                onclick={() => (cursor = f.path)}
                ondblclick={() => open(f.path)}
                title="{f.name}{f.linked ? ' (linked folder)' : ''} (double-click to open)"
              >
                <span class="glyph">📁</span><span class="name">{f.name}</span>
                {#if f.linked}<span class="link-mark" aria-label="linked">↪</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
        <div class="tiles" style:--tile="{tile}px">
          {#each listing.photos as p (p.path)}
            {@const t = thumbFor(p.path)}
            <button
              type="button"
              class="tile photo"
              class:selected={selected === p.path}
              data-key={p.path}
              onclick={() => {
                selected = p.path;
                cursor = p.path;
                picked = true;
              }}
              ondblclick={() => (preview = true)}
              title={p.name}
              use:lazy={p.path}
            >
              <span class="thumb">
                {#if t}
                  <img src={t.data} alt="" style:transform={turn(t.orientation)} draggable="false" />
                {:else if t === null}
                  <span class="no-thumb">{ext(p.path)}</span>
                {/if}
              </span>
              <span class="name">{p.name}</span>
            </button>
          {/each}
        </div>
        {#if !listing.folders.length && !listing.photos.length}
          <p class="empty">No photos or folders here.</p>
        {/if}
      {:else if loading}
        <p class="empty">Loading…</p>
      {/if}
    </div>
    {#if preview && selected}
      {@const big = details && details.path === selected && details.preview ? details.preview : thumbFor(selected)?.data}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="preview" onclick={() => (preview = false)}>
        {#if big}
          <img
            src={big}
            alt={photoName(selected)}
            style:transform={details?.path === selected && details.preview_is_thumbnail ? turn(details.orientation) : ""}
          />
        {:else}
          <p class="hint">No preview for this photo.</p>
        {/if}
        <p class="caption">{photoName(selected)} · ← → next/previous · Space or Esc to close</p>
      </div>
    {/if}
    <aside class="side">
      <div class="mini-map-wrap">
        <div class="mini-map" bind:this={mapEl}></div>
        {#if details && details.lat == null}
          <div class="no-gps">No GPS in this photo</div>
        {/if}
      </div>
      <div class="side-info">
        {#if selected}
          <p class="sel-name" title={selected}>{photoName(selected)}</p>
          {#if details && details.path === selected}
            <ExifDetails {details} />
          {:else if detailsError}
            <p class="error">{detailsError}</p>
          {:else}
            <p class="hint">Reading…</p>
          {/if}
        {:else}
          <p class="hint">Click a photo to see where it was taken and its details.</p>
        {/if}
      </div>
    </aside>
  </div>
</div>

<style>
  .inspector {
    position: fixed;
    inset: 0;
    z-index: 1500; /* above the main screen, below Help/Settings dialogs (2000) */
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 8px 16px;
    background: var(--panel);
    border-bottom: 1px solid var(--border);
    min-height: 44px;
  }
  .title {
    flex: none;
    font-size: 14px;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-text);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    flex: 1;
    white-space: nowrap;
  }
  .sep {
    color: var(--muted);
    padding: 0 2px;
  }
  button {
    font: inherit;
    color: var(--text);
    cursor: pointer;
  }
  .crumb {
    border: none;
    background: transparent;
    padding: 2px 6px;
    border-radius: 5px;
    color: var(--muted);
  }
  .crumb:hover {
    background: color-mix(in srgb, var(--muted) 15%, transparent);
    color: var(--text);
  }
  .crumb.current {
    color: var(--text);
    font-weight: 600;
  }
  .icon {
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 16px;
    padding: 2px 8px;
    border-radius: 5px;
  }
  .icon:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--muted) 15%, transparent);
  }
  .body {
    position: relative; /* the big preview covers the gallery and side pane */
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(280px, 1fr);
  }
  .grid {
    overflow-y: auto;
    padding: 14px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(var(--tile, 240px), 100%), 1fr));
    gap: 10px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    padding: 6px;
    border: 2px solid transparent;
    border-radius: 8px;
    background: transparent;
    text-align: center;
    min-width: 0;
  }
  .tile:hover {
    background: color-mix(in srgb, var(--muted) 10%, transparent);
  }
  .tile.selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .folders {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 14px;
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 260px;
    padding: 6px 12px 6px 8px;
    border: 2px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
  }
  .folder:hover {
    border-color: var(--muted);
  }
  .folder.cursor {
    outline: 2px dashed var(--accent);
    outline-offset: 2px;
  }
  .preview {
    position: absolute;
    inset: 0;
    z-index: 1100; /* above the mini map's Leaflet panes and controls */
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 20px;
    background: rgba(0, 0, 0, 0.88);
    cursor: zoom-out;
  }
  .preview img {
    width: 100%;
    height: calc(100% - 40px);
    object-fit: contain;
  }
  .preview .caption {
    color: #e6e8ee;
    font-size: 13px;
    margin: 0;
  }
  .link-mark {
    color: var(--muted);
    font-size: 13px;
  }
  .folder .glyph {
    font-size: 20px;
    line-height: 1;
  }
  .thumb {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    border-radius: 6px;
    background: color-mix(in srgb, var(--muted) 12%, transparent);
  }
  .thumb {
    overflow: hidden;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .no-thumb {
    font-size: 11px;
    color: var(--muted);
  }
  .size {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    flex: none;
  }
  .size input {
    width: 120px;
  }
  .name {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .side {
    border-left: 1px solid var(--border);
    background: var(--panel);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .mini-map-wrap {
    position: relative;
    flex: 0 0 45%;
    min-height: 200px;
    border-bottom: 1px solid var(--border);
  }
  .mini-map {
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--muted) 12%, transparent);
  }
  .no-gps {
    position: absolute;
    left: 50%;
    top: 12px;
    transform: translateX(-50%);
    z-index: 500; /* above the map tiles */
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--panel);
    border: 1px solid var(--border);
    font-size: 12px;
    color: var(--muted);
    pointer-events: none;
  }
  .side-info {
    flex: 1;
    overflow-y: auto;
    padding: 14px;
  }
  .sel-name {
    margin: 0;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .hint,
  .empty {
    color: var(--muted);
    font-size: 13px;
  }
  .error {
    color: #dc2626;
    font-size: 13px;
    margin: 0 0 10px;
  }
</style>
