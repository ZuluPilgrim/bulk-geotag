<script lang="ts">
  // A photo's GPS, date taken and camera details as a two-column list. Used by the details
  // card on the main map and by the Inspector. Details the photo doesn't have are left out.
  import type { PhotoDetails } from "$lib/types";

  let { details }: { details: PhotoDetails } = $props();

  let rows = $derived.by(() => {
    const d = details;
    const shot = [d.exposure, d.aperture, d.iso ? `ISO ${d.iso}` : null].filter(Boolean).join(" · ");
    return (
      [
        ["Camera", d.camera],
        ["Lens", d.lens],
        ["Exposure", shot],
        ["Focal length", d.focal_length],
        ["Size", d.dimensions],
        ["Software", d.software],
      ] as [string, string | null][]
    ).filter(([, v]) => !!v);
  });
</script>

<dl>
  <dt>GPS</dt>
  <dd>{details.lat != null ? `${details.lat.toFixed(5)}, ${details.lon?.toFixed(5)}` : "none"}</dd>
  <dt>Date taken</dt>
  <dd>{details.date_taken ?? "none"}</dd>
  {#each rows as [label, value] (label)}
    <dt>{label}</dt>
    <dd>{value}</dd>
  {/each}
</dl>
{#if details.issue}<p class="warn">{details.issue}</p>{/if}

<style>
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 3px 10px;
    margin: 10px 0 0;
    font-size: 12px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .warn {
    color: #d97706;
    font-weight: 600;
    font-size: 12px;
    margin: 8px 0 0;
  }
</style>
