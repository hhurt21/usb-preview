// spec.js の提案のルールを、アプリから取った本物のデータで試す (アプリを作り直さずに)。
// 使い方: node tools/try_suggestions.mjs <アプリから取ったデータ.json>
import { readFileSync } from "node:fs";
import vm from "node:vm";

const data = JSON.parse(readFileSync(process.argv[2], "utf8"));
const noop = () => {};
const el = { addEventListener: noop, textContent: "", innerHTML: "" };
const ctx = {
  console,
  document: {
    getElementById: () => el,
    querySelector: () => el,
    createElement: () => el,
    body: el,
  },
  navigator: {},
  state: {
    devices: data.devices,
    admin: true,
    names: { devices: {}, ports: {} },
  },
  mapState: { ov: { monitors: data.monitors, devices: [] }, audio: null },
  invoke: async () => null,
  fmtTime: () => "",
  setMsg: noop,
  esc: (s) => String(s),
  iconSvg: () => "",
  mapDev: () => null,
  mapName: (d) => d?.name,
  CONN: {},
  fmtBytes: (n) => (n ? `${(n / 1e12).toFixed(1)} TB` : ""),
};
vm.createContext(ctx);
vm.runInContext(
  readFileSync(new URL("../ui/spec.js", import.meta.url), "utf8") +
    `
specState.data = ${JSON.stringify(data.spec)};
specState.usage = ${JSON.stringify(data.usage)};
globalThis.result = suggestions();`,
  ctx,
);
for (const s of ctx.result) console.log(`${s.level.padEnd(5)} | ${s.title}`);
