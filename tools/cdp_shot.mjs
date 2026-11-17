// 試験用ビルドの画面を、色の設定 (明るい/暗い) と幅を変えて撮る。アプリの画面の中身だけを撮る。
// 使い方: node tools/cdp_shot.mjs <出力.png> <light|dark> <幅px> [スクロールする要素の CSS セレクター]
import { writeFileSync } from "node:fs";

const [out, scheme = "light", width = "0", selector = ""] =
  process.argv.slice(2);
const list = await (await fetch("http://127.0.0.1:9333/json")).json();
const page = list.find((p) => p.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
ws.onmessage = (m) => {
  const d = JSON.parse(m.data);
  if (pending.has(d.id)) {
    pending.get(d.id)(d);
    pending.delete(d.id);
  }
};
const call = (method, params = {}) =>
  new Promise((r) => {
    const n = ++id;
    pending.set(n, r);
    ws.send(JSON.stringify({ id: n, method, params }));
  });

await call("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-color-scheme", value: scheme }],
});
const w = Number(width);
if (w > 0)
  await call("Emulation.setDeviceMetricsOverride", {
    width: w,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
else await call("Emulation.clearDeviceMetricsOverride");
await new Promise((r) => setTimeout(r, 600));
if (selector) {
  await call("Runtime.evaluate", {
    expression: `document.querySelector(${JSON.stringify(selector)})?.scrollIntoView({block:"start"}); window.scrollBy(0,-10); drawLines && drawLines();`,
  });
  await new Promise((r) => setTimeout(r, 400));
}
const shot = await call("Page.captureScreenshot", { format: "png" });
writeFileSync(out, Buffer.from(shot.result.data, "base64"));
// 元に戻す
await call("Emulation.setEmulatedMedia", { features: [] });
await call("Emulation.clearDeviceMetricsOverride");
console.log("saved", out);
ws.close();
process.exit(0);
