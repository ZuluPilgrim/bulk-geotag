/** What `inspect_photo` returns: a photo's current GPS, date taken, camera details and a preview. */
export type PhotoDetails = {
  path: string;
  name: string;
  lat: number | null;
  lon: number | null;
  date_taken: string | null; // "YYYY-MM-DD HH:MM:SS"
  camera: string | null;
  lens: string | null;
  exposure: string | null; // "1/250 s"
  aperture: string | null; // "f/2.8"
  iso: number | null;
  focal_length: string | null; // "35 mm (53 mm equiv.)"
  dimensions: string | null; // "6240 × 4160"
  software: string | null;
  orientation: number;
  preview: string | null;
  preview_is_thumbnail: boolean;
  issue: string | null;
};
