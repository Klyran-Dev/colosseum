#!/usr/bin/env node
// Compare two directories of e2e style snapshots (ADR-0428).
//   node scripts/diff_style_snapshots.mjs <before> <after> [--limit N]
//   node scripts/diff_style_snapshots.mjs <before> <after> --props [--expect a,b,c]
//   node scripts/diff_style_snapshots.mjs --coverage <dir> [stylesheet]
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const PROPS = ["color","backgroundColor","borderTopColor","borderRightColor","borderBottomColor","borderLeftColor","borderTopWidth","borderLeftWidth","borderTopLeftRadius","borderTopRightRadius","borderBottomLeftRadius","borderBottomRightRadius","boxShadow","fontSize","fontWeight","lineHeight","letterSpacing","textTransform","paddingTop","paddingRight","paddingBottom","paddingLeft","marginTop","marginRight","marginBottom","marginLeft","display","opacity","gridTemplateColumns","gap","position","textDecorationLine","outlineStyle","size"];
const SNAPSHOT_FORMAT = 5;
const load = (dir) => {
  const snaps = Object.fromEntries(readdirSync(dir).filter((f) => f.endsWith(".json")).map((f) => [f, JSON.parse(readFileSync(join(dir, f), "utf8"))]));
  const stale = Object.entries(snaps).filter(([, snap]) => snap.format !== SNAPSHOT_FORMAT).map(([name]) => name);
  if (stale.length) {
    console.error(`${dir}: ${stale.length} snapshot(s) are not format ${SNAPSHOT_FORMAT} (e.g. ${stale[0]}). Re-take this baseline with scripts/web_style_snapshot.sh; snapshots are not comparable across formats.`);
    process.exit(2);
  }
  return snaps;
};

function coverage(dir, stylesheetPath = "apps/web/src/styles.css") {
  const rendered = new Set();
  const snaps = load(dir);
  Object.values(snaps).forEach((s) => s.classes.forEach((c) => rendered.add(c)));
  const css = readFileSync(stylesheetPath, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const styled = new Set([...css.matchAll(/\.([a-zA-Z][\w-]*)/g)].map((m) => m[1]));
  const never = [...styled].filter((c) => !rendered.has(c)).sort();
  console.log(`journeys captured: ${Object.keys(snaps).length}`);
  console.log(`classes with CSS: ${styled.size}; rendered by at least one journey: ${styled.size - never.length}; never rendered: ${never.length}`);
  console.log(never.join(" "));
}

function propsReport(a, b, expect) {
  const A = load(a), B = load(b); const hist = {}; const unexpected = {}; let total = 0;
  for (const [file, x] of Object.entries(A)) {
    const y = B[file]; if (!y) continue;
    for (const theme of ["dark", "light"]) {
      const p = x[theme], q = y[theme]; if (p.ids.length !== q.ids.length) continue;
      for (let i = 0; i < p.ids.length; i++) {
        const sa = p.strs[p.ids[i]], sb = q.strs[q.ids[i]]; if (sa === sb) continue;
        const pa = sa.split("|"), pb = sb.split("|");
        total++;
        pa.forEach((v, j) => {
          if (v === pb[j]) return; const name = PROPS[j]; hist[name] = (hist[name] || 0) + 1;
          if (expect && !expect.has(name)) { const k = `${name} ${v}->${pb[j]} on ${p.keys[i]} [${file.slice(0, 48)}]`; unexpected[k] = (unexpected[k] || 0) + 1; }
        });
      }
    }
  }
  console.log(`${total} element-diffs; property histogram:`);
  Object.entries(hist).sort((m, n) => n[1] - m[1]).forEach(([k, v]) => console.log(`  ${String(v).padStart(7)}  ${k}${expect && !expect.has(k) ? "   <-- UNEXPECTED" : ""}`));
  const bad = Object.entries(unexpected).sort((m, n) => n[1] - m[1]);
  console.log(`unexpected-property changes: ${bad.length} distinct`);
  bad.slice(0, 25).forEach(([k, v]) => console.log(`  ${v}x ${k}`));
  process.exitCode = bad.length ? 1 : 0;
}

function diff(a, b, limit) {
  const A = load(a), B = load(b); const agg = {}; let total = 0; const issues = [];
  for (const [file, x] of Object.entries(A)) {
    const y = B[file];
    if (!y) { issues.push(`missing in after: ${file}`); continue; }
    if (x.settled === false || y.settled === false) issues.push(`unsettled capture (page still changing at the 4s cap): ${file}`);
    for (const theme of ["dark", "light"]) {
      const p = x[theme], q = y[theme];
      if (p.ids.length !== q.ids.length) { issues.push(`${file} ${theme}: element count ${p.ids.length} -> ${q.ids.length}`); continue; }
      for (let i = 0; i < p.ids.length; i++) {
        const sa = p.strs[p.ids[i]], sb = q.strs[q.ids[i]]; if (sa === sb) continue;
        const pa = sa.split("|"), pb = sb.split("|"), ch = [];
        pa.forEach((v, j) => { if (v !== pb[j]) ch.push(`${PROPS[j]} ${v}->${pb[j]}`); });
        const key = `${theme} ${p.keys[i]} :: ${ch.join("; ")}`; agg[key] = (agg[key] || 0) + 1; total++;
      }
    }
  }
  const lines = Object.entries(agg).sort((m, n) => n[1] - m[1]).map(([k, v]) => `${v}x ${k}`);
  console.log(`${Object.keys(A).length} journeys compared: ${total} element-diffs, ${lines.length} distinct, ${issues.length} structural issues`);
  issues.slice(0, 20).forEach((l) => console.log(`! ${l}`));
  lines.slice(0, limit).forEach((l) => console.log(l));
  if (lines.length > limit) console.log(`… +${lines.length - limit} more`);
  process.exitCode = total || issues.length ? 1 : 0;
}

const args = process.argv.slice(2);
if (args[0] === "--coverage") {
  if (!args[1] || !existsSync(args[1])) { console.error("usage: --coverage <dir> [stylesheet]"); process.exit(2); }
  coverage(args[1], args[2]);
} else if (args.length >= 2 && args.includes("--props")) {
  const ei = args.indexOf("--expect");
  propsReport(args[0], args[1], ei > -1 ? new Set(args[ei + 1].split(",")) : null);
} else if (args.length >= 2) {
  const li = args.indexOf("--limit"); diff(args[0], args[1], li > -1 ? Number(args[li + 1]) : 60);
} else { console.error("usage: diff_style_snapshots.mjs <before> <after> [--limit N] | --coverage <dir>"); process.exit(2); }
