// 全体図タブ。index.html の中の state・invoke・listen・esc・up・displayName・statusBadge・openDetail などを使う。

const CONN = {
  0: "VGA",
  1: "S ビデオ",
  2: "コンポジット",
  3: "コンポーネント",
  4: "DVI",
  5: "HDMI",
  6: "内蔵 (LVDS)",
  8: "D 端子",
  9: "SDI",
  10: "DisplayPort",
  11: "内蔵 DisplayPort",
  12: "UDI",
  13: "内蔵 UDI",
  14: "SDTV",
  15: "Miracast（無線）",
  16: "間接（有線）",
  17: "仮想",
  18: "USB-C（DisplayPort）",
  [-2147483648]: "内蔵",
  2147483648: "内蔵",
};

/** 全体図に出さない、Windows の中の部品の種類 */
const INTERNAL_CLASSES = new Set([
  "System",
  "Processor",
  "Computer",
  "SoftwareDevice",
  "SoftwareComponent",
  "Volume",
  "VolumeSnapshot",
  "PrintQueue",
  "AudioProcessingObject",
  "SecurityDevices",
  "Firmware",
  "Extension",
  "LegacyDriver",
  "SCSIAdapter",
  "HDC",
  "HIDClass",
  "Keyboard",
  "Mouse",
  "MEDIA",
  "Monitor",
  "USB",
  "Bluetooth",
  "Display",
  "Net",
  "DiskDrive",
  "AudioEndpoint",
  "XnaComposite",
  "Ports",
  "SoftwareComponent",
]);

const mapState = {
  ov: null, // get_overview の結果
  extra: null, // get_overview_extra の結果
  byId: new Map(), // 大文字 ID → 機器
  kids: new Map(), // 大文字 ID → 子の配列
  loading: false,
};

function mapDev(id) {
  return id ? mapState.byId.get(up(id)) : undefined;
}

function* ancestors(d) {
  let cur = d;
  for (let i = 0; i < 16 && cur; i++) {
    yield cur;
    cur = mapDev(cur.parent);
  }
}

/** 本物の部品か、Windows やアプリが作った仮想の機器か */
function isVirtualDev(d) {
  for (const a of ancestors(d)) {
    const u = up(a.id);
    if (u.startsWith("SWD\\MMDEVAPI")) continue;
    if (
      u.startsWith("ROOT\\") ||
      u.startsWith("SWD\\") ||
      u.startsWith("LGHUBDEVICE\\") ||
      u.startsWith("HTREE\\")
    )
      return true;
    if (u.startsWith("PCI\\") || u.startsWith("ACPI\\")) return false;
  }
  return false;
}

function descendants(d, depth = 0, out = []) {
  if (depth > 10) return out;
  for (const c of mapState.kids.get(up(d.id)) || []) {
    out.push(c);
    descendants(c, depth + 1, out);
  }
  return out;
}

/** 複合機器の部品 (&MI_ や HID) から、もとの USB 機器 (ポートにさした物) までさかのぼる */
function topUsbDevice(d) {
  let last = null;
  for (const a of ancestors(d)) {
    const u = up(a.id);
    if (u.startsWith("USB\\") && !u.includes("&MI_") && !u.includes("&IG_") && !u.includes("ROOT_HUB") && !isUsbChild(a))
      return a;
    if (u.startsWith("BTHENUM\\DEV_") || u.startsWith("BTHLE\\DEV_")) return a;
    last = a;
  }
  return null;
}

function isUsbChild(d) {
  const p = mapDev(d.parent);
  if (!p) return false;
  const u = up(p.id);
  return u.startsWith("USB\\") && !u.includes("ROOT_HUB") && !p.is_hub;
}

function mapName(d) {
  if (!d) return "";
  const alias = state.names.devices[up(d.id)];
  if (alias) return alias;
  const generic =
    /^(USB Composite Device|USB 入力デバイス|HID キーボード デバイス|HID 準拠マウス)$/i;
  if (d.bus_name && generic.test(d.name)) return d.bus_name;
  return d.name;
}

/** USB 機器が何かを、部品の種類から当てる */
function guessUsbIcon(d) {
  const ds = [d, ...descendants(d)];
  const has = (re) => ds.some((x) => re.test(x.class || ""));
  const nm = ((d.bus_name || "") + " " + d.name).toLowerCase();
  if (has(/^Bluetooth$/)) return "bluetooth";
  if (
    has(/^XnaComposite$/) ||
    ds.some((x) => /ゲーム コントローラー/.test(x.name)) ||
    /controller|gamepad|game ?pad|joystick|コントローラー/i.test(nm)
  )
    return "gamepad";
  if (has(/^Keyboard$/) && !has(/^Mouse$/)) return "keyboard";
  if (has(/^Keyboard$/) && has(/^Mouse$/))
    return /mouse|マウス/.test(nm) ? "mouse" : "keyboard";
  if (has(/^Mouse$/)) return "mouse";
  if (has(/^(Camera|Image)$/)) return "camera";
  if (has(/^(DiskDrive|CDROM)$/)) return "usbdrive";
  if (has(/^(MEDIA|AudioEndpoint)$/))
    return /buds|earphone|イヤホン/.test(nm)
      ? "earbuds"
      : /headset|ヘッド/.test(nm)
        ? "headphones"
        : "audioif";
  if (has(/^Net$/)) return "ethernet";
  if (has(/^Printer$/)) return "printer";
  if (d.is_hub) return "hub";
  return "usb";
}

function audioIcon(d) {
  const n = d.name.toLowerCase();
  if (up(d.id).includes("{0.0.1.")) return "mic";
  if (/ヘッドホン|ヘッドセット|headphone|headset|buds|イヤホン/.test(n))
    return "headphones";
  return "speaker";
}

function fmtBytes(n) {
  if (!n) return "";
  const tb = n / 1e12;
  return tb >= 1
    ? `${tb.toFixed(tb >= 10 ? 0 : 1)} TB`
    : `${Math.round(n / 1e9)} GB`;
}

function netExtra(d) {
  return (mapState.extra?.nets || []).find((x) => up(x.id) === up(d.id));
}
function diskExtra(d) {
  return (mapState.extra?.disks || []).find((x) => up(x.id) === up(d.id));
}

// ---------- 仕分け ----------
function categorize() {
  const all = mapState.ov.devices;
  const q = document.getElementById("mText").value.trim().toLowerCase();
  const showVirtual = document.getElementById("mVirtual").checked;
  const match = (d) =>
    !q ||
    [mapName(d), d.name, d.bus_name, d.manufacturer].some((s) =>
      String(s || "")
        .toLowerCase()
        .includes(q),
    );
  const cats = {
    screen: { title: "画面", icon: "monitor", items: [], virtual: [] },
    usb: { title: "USB", icon: "usb", items: [], virtual: [] },
    audio: { title: "音", icon: "sound", items: [], virtual: [] },
    input: { title: "入力", icon: "input", items: [], virtual: [] },
    bt: { title: "Bluetooth", icon: "bluetooth", items: [], virtual: [] },
    net: { title: "ネットワーク", icon: "ethernet", items: [], virtual: [] },
    store: { title: "保存", icon: "ssd", items: [], virtual: [] },
    other: { title: "その他", icon: "other", items: [], virtual: [] },
  };
  const used = new Set();
  const add = (cat, d, item) => {
    used.add(up(d.id));
    if (
      !match(d) &&
      !(item.matchText && item.matchText.toLowerCase().includes(q))
    )
      return;
    (isVirtualDev(d) ? cats[cat].virtual : cats[cat].items).push({
      dev: d,
      ...item,
    });
  };

  // 画面: GPU (画面の絵は別に描く)
  for (const d of all.filter((x) => x.class === "Display")) {
    const mons = mapState.ov.monitors.filter(
      (m) => up(m.adapter_id) === up(d.id),
    );
    add("screen", d, {
      icon: "gpu",
      sub: mons.length ? `画面 ${mons.length} 台` : "画面なし",
    });
  }
  for (const d of all.filter((x) => x.class === "Monitor")) used.add(up(d.id));

  // USB: ポートにさした物 (ハブ・部品・ルートハブは除く)
  for (const d of all) {
    const u = up(d.id);
    if (
      !u.startsWith("USB\\") ||
      u.includes("&MI_") ||
      u.includes("&IG_") ||
      isUsbChild(d) ||
      u.includes("ROOT_HUB") ||
      d.is_hub
    )
      continue;
    const row = state.devices.find((x) => up(x.id) === u);
    const flap = row && isFlap(row);
    add("usb", d, { icon: guessUsbIcon(d), sub: placeText(row || d), flap });
  }

  // 音: 出力と入力 (エンドポイント)
  for (const d of all.filter((x) => x.class === "AudioEndpoint")) {
    const out = !up(d.id).includes("{0.0.1.");
    add("audio", d, {
      icon: audioIcon(d),
      sub: out ? "出力（鳴らす）" : "入力（録る）",
    });
  }

  // 入力: キーボード・マウス・ゲームパッドを、もとの機器ごとにまとめる
  const inputs = new Map();
  for (const d of all) {
    let kind = null;
    if (d.class === "Keyboard") kind = "キーボード";
    else if (d.class === "Mouse") kind = "マウス";
    else if (d.class === "XnaComposite" || /ゲーム コントローラー/.test(d.name))
      kind = "ゲームパッド";
    if (!kind) continue;
    used.add(up(d.id));
    const top = topUsbDevice(d) || d;
    const k = up(top.id);
    if (!inputs.has(k)) inputs.set(k, { top, kinds: new Set() });
    inputs.get(k).kinds.add(kind);
  }
  for (const { top, kinds } of inputs.values()) {
    const ks = [...kinds];
    const icon = up(top.id).startsWith("USB\\")
      ? guessUsbIcon(top)
      : ks.includes("ゲームパッド")
      ? "gamepad"
      : ks.length === 1 && ks[0] === "マウス"
        ? "mouse"
        : "keyboard";
    const via = up(top.id).startsWith("BTH")
      ? "Bluetooth"
      : up(top.id).startsWith("USB\\")
        ? "USB"
        : "";
    add("input", top, {
      icon,
      sub: ks.join("・") + (via ? `（${via}）` : ""),
      matchText: ks.join(" "),
    });
  }

  // Bluetooth: アダプターとペアリングした機器だけ
  for (const d of all) {
    const u = up(d.id);
    if (
      d.class === "Bluetooth" &&
      (u.startsWith("USB\\") || u.startsWith("PCI\\"))
    )
      add("bt", d, { icon: "bluetooth", sub: "アダプター" });
    else if (u.startsWith("BTHENUM\\DEV_") || u.startsWith("BTHLE\\DEV_")) {
      const nm = d.name.toLowerCase();
      const icon = /buds|ze3000|earphone|イヤホン|headphone|ヘッド/.test(nm)
        ? "earbuds"
        : /kb|key|キーボード|ak8/.test(nm)
          ? "keyboard"
          : /mouse|マウス/.test(nm)
            ? "mouse"
            : "bluetooth";
      add("bt", d, {
        icon,
        sub: u.startsWith("BTHLE") ? "ペアリング済み（LE）" : "ペアリング済み",
      });
    }
  }

  // ネットワーク
  for (const d of all.filter((x) => x.class === "Net")) {
    const e = netExtra(d);
    const wifi = /wi-?fi|wireless|802\.11|無線/i.test(d.name);
    const st = e
      ? e.status === "Up"
        ? `つながっている ${e.speed || ""}`
        : e.status === "Disconnected"
          ? "つながっていない"
          : e.status
      : "";
    const item = { icon: wifi ? "wifi" : "ethernet", sub: st };
    used.add(up(d.id));
    if (!match(d)) continue;
    const virtual = e ? !e.physical : isVirtualDev(d);
    (virtual ? cats.net.virtual : cats.net.items).push({ dev: d, ...item });
  }

  // 保存
  for (const d of all.filter(
    (x) => x.class === "DiskDrive" || x.class === "CDROM",
  )) {
    const e = diskExtra(d);
    const viaUsb = [...ancestors(d)].some((a) => up(a.id).startsWith("USB\\"));
    const kind = /nvme|ssd/i.test(d.id + d.name) ? "ssd" : "hdd";
    add("store", d, {
      icon: kind,
      sub: [
        fmtBytes(e?.size),
        viaUsb ? "USB" : /NVME/.test(up(d.id)) ? "NVMe" : e?.iface,
      ]
        .filter(Boolean)
        .join("・"),
    });
  }

  // その他: どこにも入らない、Windows の中の部品でもない物 (カメラ・プリンター・シリアルなど)
  for (const d of all) {
    if (used.has(up(d.id))) continue;
    const u = up(d.id);
    if (
      u.startsWith("BTHLEDEVICE\\") ||
      u.startsWith("BTHENUM\\") ||
      u.startsWith("BTH\\") ||
      u.startsWith("HID\\") ||
      u.startsWith("USB\\")
    )
      continue;
    if (!d.class || INTERNAL_CLASSES.has(d.class)) continue;
    const icon = /Camera|Image/.test(d.class)
      ? "camera"
      : /Printer/.test(d.class)
        ? "printer"
        : /Ports/.test(d.class)
          ? "serial"
          : "chip";
    add("other", d, { icon, sub: d.class });
  }
  // シリアル (COM) は「その他」に出す (Bluetooth 経由の物も)
  for (const d of all.filter((x) => x.class === "Ports"))
    add("other", d, { icon: "serial", sub: "シリアル（COM）" });

  // Windows の中の部品 (数だけ)
  const internal = all.filter((d) => !used.has(up(d.id)));
  for (const c of Object.values(cats))
    if (!showVirtual) c.virtualHidden = c.virtual.length;
  return { cats, internal: internal.filter(match), showVirtual };
}

// ---------- 描く ----------
function devRow(it) {
  const d = it.dev;
  const st = d.status !== "ok" ? statusBadge(d) : "";
  return `<button class="mrow ${it.flap ? "flap" : ""}" data-open="${esc(d.id)}" title="${esc(d.name)}">
    <span class="mic-wrap">${iconSvg(it.icon, 30)}</span>
    <span class="mtext"><span class="mname">${esc(mapName(d))}</span>${it.sub ? `<span class="sub">${esc(it.sub)}</span>` : ""}${it.flap ? `<span class="flapnote">くり返し外れています</span>` : ""}</span>
    ${st}
  </button>`;
}

function cardHtml(key, c, showVirtual, extraTop = "", countText = null) {
  const list = c.items.concat(showVirtual ? c.virtual : []);
  const vnote =
    !showVirtual && c.virtual.length
      ? `<div class="sub mvnote">仮想の機器 ${c.virtual.length} 個（上の「仮想の機器も出す」で表示）</div>`
      : "";
  return `<section class="mcard" data-card="${key}" style="grid-area:${key}">
    <h3><span class="mic-wrap big">${iconSvg(c.icon, 34)}</span>${esc(c.title)}<span class="mcount">${countText ?? `${c.items.length} 台`}</span></h3>
    ${extraTop}
    <div class="mlist">${list.map(devRow).join("") || `<div class="sub">ありません</div>`}</div>
    ${vnote}
  </section>`;
}

function monitorsHtml() {
  const ms = mapState.ov.monitors.filter((m) => m.width > 0);
  if (!ms.length) return `<div class="sub">画面の情報を取れませんでした</div>`;
  const minX = Math.min(...ms.map((m) => m.x)),
    minY = Math.min(...ms.map((m) => m.y));
  const maxX = Math.max(...ms.map((m) => m.x + m.width)),
    maxY = Math.max(...ms.map((m) => m.y + m.height));
  const W = maxX - minX,
    H = maxY - minY;
  const gpuName = (id) => {
    const g = mapDev(id);
    return g ? mapName(g) : "";
  };
  const mons = ms.map((m) => {
    const HH = H * 1.22;
    const left = ((m.x - minX) / W) * 100,
      top = ((m.y - minY) / HH) * 100;
    const w = (m.width / W) * 100,
      h = (m.height / HH) * 100;
    const mon = mapDev(m.monitor_id);
    const nm =
      (mon && state.names.devices[up(mon.id)]) ||
      m.name ||
      (mon ? mon.name : "画面");
    return `<button class="mon" style="left:${left}%;top:${top}%;width:${w}%;height:${h}%" ${mon ? `data-open="${esc(mon.id)}"` : ""}>
      <span class="mon-screen">
        <span class="mon-name">${esc(nm)}</span>
        <span class="mon-spec">${m.width}×${m.height}・${Math.round(m.hz)}Hz</span>
        <span class="mon-spec">${esc(CONN[m.connection] ?? "不明なつなぎ方")}${m.primary ? "・<b>メイン</b>" : ""}</span>
        <span class="mon-gpu">${esc(gpuName(m.adapter_id))}</span>
      </span>
    </button>`;
  });
  return `<div class="mons-wrap"><div class="mons" style="aspect-ratio:${W} / ${H * 1.22};width:min(100%, ${Math.round((300 * W) / (H * 1.22))}px)">${mons.join("")}</div></div>
    <p class="sub">Windows の画面設定と同じ並び方です。モニターを押すと、くわしく見られます。</p>`;
}

function pcHtml() {
  const pc = mapState.ov.pc;
  const gpus = mapState.ov.devices.filter(
    (d) => d.class === "Display" && !isVirtualDev(d),
  );
  const spec = [
    pc.cpu
      ? [
          `cpu`,
          pc.cpu.replace(/\s+/g, " "),
          pc.threads ? `${pc.threads} スレッド` : "",
        ]
      : null,
    pc.memory_gb
      ? [`memory`, `メモリ ${Math.round(pc.memory_gb)} GB`, ""]
      : null,
    ...gpus.map((g) => [`gpu`, mapName(g), "GPU"]),
  ].filter(Boolean);
  return `<section class="mcard mpc" data-card="pc" style="grid-area:pc">
    <div class="pc-pic">${iconSvg("pc", 96)}</div>
    <h3 class="pc-title">この PC</h3>
    <div class="pc-spec">${spec.map(([ic, a, b]) => `<div class="pc-line">${iconSvg(ic, 26)}<span><b>${esc(a)}</b>${b ? `<span class="sub"> ${esc(b)}</span>` : ""}</span></div>`).join("")}</div>
  </section>`;
}

function renderMap() {
  if (!mapState.ov) return;
  const { cats, internal, showVirtual } = categorize();
  const internalByClass = {};
  for (const d of internal)
    (internalByClass[d.class || "（種類なし）"] ||= []).push(d);
  const internalHtml = `<section class="mcard" data-card="internal" style="grid-area:internal">
    <h3><span class="mic-wrap big">${iconSvg("chip", 34)}</span>Windows の中の部品<span class="mcount">${internal.length} 個</span></h3>
    <p class="sub">マザーボードの部品・ドライバーの部品・ボリュームなど、ふだん気にしなくてよい物です。</p>
    ${Object.entries(internalByClass)
      .sort((a, b) => b[1].length - a[1].length)
      .map(
        ([cls, ds]) =>
          `<details><summary>${esc(cls)}（${ds.length}）</summary><div class="mlist">${ds.map((d) => devRow({ dev: d, icon: "chip", sub: "" })).join("")}</div></details>`,
      )
      .join("")}
  </section>`;
  document.getElementById("map").innerHTML =
    `<svg class="mlines" id="mlines" aria-hidden="true"></svg>
    ${pcHtml()}
    ${cardHtml("screen", cats.screen, showVirtual, monitorsHtml(), `${mapState.ov.monitors.length} 台`)}
    ${cardHtml("usb", cats.usb, showVirtual)}
    ${cardHtml("audio", cats.audio, showVirtual)}
    ${cardHtml("input", cats.input, showVirtual)}
    ${cardHtml("bt", cats.bt, showVirtual)}
    ${cardHtml("net", cats.net, showVirtual)}
    ${cardHtml("store", cats.store, showVirtual)}
    ${cardHtml("other", cats.other, showVirtual)}
    ${internalHtml}`;
  const n = mapState.ov.devices.length;
  document.getElementById("mInfo").textContent =
    `全部で ${n} 個の機器（${fmtTime(Date.now())} に読みました）`;
  requestAnimationFrame(drawLines);
}

/** この PC から各箱へ線を引く (横に並ぶ広さがあるときだけ) */
function drawLines() {
  const map = document.getElementById("map");
  const svg = document.getElementById("mlines");
  const pc = map?.querySelector('[data-card="pc"]');
  if (!svg || !pc) return;
  const narrow =
    getComputedStyle(map).getPropertyValue("--narrow").trim() === "1";
  const base = map.getBoundingClientRect();
  svg.setAttribute("width", base.width);
  svg.setAttribute("height", base.height);
  if (narrow) {
    svg.innerHTML = "";
    return;
  }
  const p = pc.getBoundingClientRect();
  const pcx = p.left + p.width / 2 - base.left,
    pcy = p.top + p.height / 2 - base.top;
  const paths = [];
  for (const card of map.querySelectorAll(".mcard:not(.mpc)")) {
    if (card.dataset.card === "internal" || card.dataset.card === "other") continue;
    const r = card.getBoundingClientRect();
    const cx = Math.min(
      Math.max(pcx, r.left - base.left + 24),
      r.right - base.left - 24,
    );
    const cy = Math.min(
      Math.max(pcy, r.top - base.top + 24),
      r.bottom - base.top - 24,
    );
    const x =
      Math.abs(cx - pcx) > Math.abs(cy - pcy)
        ? cx > pcx
          ? r.left - base.left
          : r.right - base.left
        : cx;
    const y =
      Math.abs(cx - pcx) > Math.abs(cy - pcy)
        ? cy
        : cy > pcy
          ? r.top - base.top
          : r.bottom - base.top;
    paths.push(
      `<path d="M${pcx} ${pcy} L${x} ${y}"/><circle cx="${x}" cy="${y}" r="5"/>`,
    );
  }
  svg.innerHTML = paths.join("");
}

async function loadMap(withExtra) {
  if (mapState.loading) return;
  mapState.loading = true;
  try {
    const ov = await invoke("get_overview");
    mapState.ov = ov;
    state.all = ov.devices;
    mapState.byId = new Map(ov.devices.map((d) => [up(d.id), d]));
    mapState.kids = new Map();
    for (const d of ov.devices) {
      const p = up(d.parent);
      if (!mapState.kids.has(p)) mapState.kids.set(p, []);
      mapState.kids.get(p).push(d);
    }
    renderMap();
    if (withExtra || !mapState.extra) {
      invoke("get_overview_extra")
        .then((e) => {
          mapState.extra = e;
          renderMap();
        })
        .catch(() => {});
    }
  } catch (e) {
    document.getElementById("mInfo").textContent = "読めませんでした: " + e;
  } finally {
    mapState.loading = false;
  }
}

const mapVisible = () => !document.getElementById("tab-map").hidden;
let mapTimer = null;
listen("devices-changed", () => {
  if (!mapVisible()) return;
  clearTimeout(mapTimer);
  mapTimer = setTimeout(() => loadMap(false), 800);
});
document.getElementById("map").addEventListener("click", (e) => {
  const b = e.target.closest("[data-open]");
  if (b) openDetail(b.dataset.open);
});
document.getElementById("mVirtual").addEventListener("change", renderMap);
document.getElementById("mText").addEventListener("input", renderMap);
document
  .getElementById("mReload")
  .addEventListener("click", () => loadMap(true));
new ResizeObserver(() => {
  if (mapVisible()) drawLines();
}).observe(document.getElementById("map"));
