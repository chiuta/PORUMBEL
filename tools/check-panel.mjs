// Syntax check of the control panel's inline script, and a consistency check that every
// language has the same keys as English. Run: node tools/check-panel.mjs
import { readFileSync } from "node:fs";
const html = readFileSync(new URL("../src/panel.html", import.meta.url), "utf8");
const js = html.match(/<script>([\s\S]*)<\/script>/)[1];
new Function(js); // throws on a syntax error
const T = new Function(js.slice(js.indexOf("const T="), js.indexOf("const q=")) + "; return T;")();
const en = Object.keys(T.en).sort().join();
let bad = 0;
for (const [lang, d] of Object.entries(T)) {
  if (Object.keys(d).sort().join() !== en) { console.error(`panel: language "${lang}" has different keys than "en"`); bad++; }
}
for (const opt of Object.keys(T)) if (!html.includes(`<option value="${opt}">`)) { console.error(`panel: no <option> for "${opt}"`); bad++; }
console.log(`panel script ok, ${Object.keys(T).length} languages${bad ? `, ${bad} problem(s)` : ""}`);
process.exit(bad ? 1 : 0);
