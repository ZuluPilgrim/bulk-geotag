// Checks docs/USER_GUIDE.md the way the in-app Help shows it:
//  - every in-page link (#anchor) matches a heading id
//  - every local image (e.g. images/screenshot.png) exists in docs/
// Usage: npm run check:guide   (exits 1 if anything is broken)
import { readFileSync, existsSync } from "node:fs";
import { Marked } from "marked";
import { gfmHeadingId } from "marked-gfm-heading-id";

const guide = new URL("../docs/USER_GUIDE.md", import.meta.url);
const md = readFileSync(guide, "utf8");
const html = new Marked(gfmHeadingId()).parse(md, { async: false });

const ids = new Set([...html.matchAll(/ id="([^"]+)"/g)].map((m) => m[1]));
const anchors = [...md.matchAll(/\]\(#([^)\s]+)\)/g)].map((m) => m[1]);
const badAnchors = anchors.filter((a) => !ids.has(decodeURIComponent(a)));

const images = [...md.matchAll(/!\[[^\]]*\]\(([^)\s]+)\)/g)].map((m) => m[1]).filter((src) => !/^https?:/.test(src));
const badImages = images.filter((src) => !existsSync(new URL(`../docs/${src}`, import.meta.url)));

for (const a of badAnchors) console.log(`  broken link: #${a} (no heading with that id)`);
for (const i of badImages) console.log(`  missing image: docs/${i}`);
const ok = badAnchors.length === 0 && badImages.length === 0;
console.log(`${anchors.length} anchor links, ${images.length} image(s): ${ok ? "all OK" : "PROBLEMS above"}`);
process.exit(ok ? 0 : 1);
