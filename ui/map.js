// 全体図タブ。index.html の中の state・invoke・listen・esc・up・statusBadge・openDetail・placeText・isFlap・fmtTime を使う。

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

/** 全体図の箱に入れない、Windows の中の部品の種類 */
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
]);

const mapState = {
  ov: null, // get_overview の結果
  extra: null, // get_overview_extra の結果 (ディスク・ネットワークの詳細)
  audio: null, // get_audio_state の結果 (既定の機器・音の大きさ・アプリ)
  audioSig: "",
  byId: new Map(), // 大文字 ID → 機器
  kids: new Map(), // 大文字 ID → 子の配列
  loading: false,
};

const mapDev = (id) => (id ? mapState.byId.get(up(id)) : undefined);
const kidsOf = (d) => mapState.kids.get(up(d.id)) || [];

function* ancestors(d) {
  let cur = d;
  for (let i = 0; i < 16 && cur; i++) {
    yield cur;
    cur = mapDev(cur.parent);
  }
}

/** 本物の部品か、Windows やアプリが作った仮想の機器か。
 *  親をたどって、本物の部品 (PCI・ACPI) に着く前に Windows が作った機器 (ROOT・SWD) に着いたら仮想 */
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
  for (const c of kidsOf(d)) {
    out.push(c);
    descendants(c, depth + 1, out);
  }
  return out;
}

const isRootHub = (d) => up(d.id).includes("ROOT_HUB");
const isUsbHub = (d) =>
  up(d.id).startsWith("USB\\") && (d.is_hub || isRootHub(d));

/** 親がふつうの USB 機器 (ハブでない) なら、その中の部品 */
function isUsbChild(d) {
  const p = mapDev(d.parent);
  if (!p) return false;
  return up(p.id).startsWith("USB\\") && !isUsbHub(p);
}

/** ポートにさした USB 機器そのもの (部品・ハブではない) か */
function isPluggedUsb(d) {
  const u = up(d.id);
  return (
    u.startsWith("USB\\") &&
    !u.includes("&MI_") &&
    !u.includes("&IG_") &&
    !isUsbHub(d) &&
    !isUsbChild(d)
  );
}

/** 部品 (&MI_・HID など) から、もとの機器 (ポートにさした物・ペアリングした物) までさかのぼる */
function topDevice(d) {
  for (const a of ancestors(d)) {
    const u = up(a.id);
    if (u.startsWith("USB\\") && isPluggedUsb(a)) return a;
    if (u.startsWith("BTHENUM\\DEV_") || u.startsWith("BTHLE\\DEV_")) return a;
  }
  return null;
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

/** USB 機器が何かを、部品の種類と名前から当てる */
function guessUsbIcon(d) {
  const ds = [d, ...descendants(d)];
  const has = (re) => ds.some((x) => re.test(x.class || ""));
  const nm = ((d.bus_name || "") + " " + d.name).toLowerCase();
  if (isUsbHub(d)) return "hub";
  if (has(/^Bluetooth$/)) return "bluetooth";
  if (
    has(/^XnaComposite$/) ||
    ds.some((x) => /ゲーム コントローラー/.test(x.name)) ||
    /controller|gamepad|game ?pad|joystick|コントローラー/i.test(nm)
  )
    return "gamepad";
  if (has(/^(MEDIA|AudioEndpoint)$/))
    return /buds|earphone|イヤホン/.test(nm)
      ? "earbuds"
      : /headset|headphone|ヘッド/.test(nm)
        ? "headphones"
        : "audioif";
  if (has(/^Keyboard$/) && has(/^Mouse$/))
    return /mouse|マウス/.test(nm) ? "mouse" : "keyboard";
  if (has(/^Keyboard$/)) return "keyboard";
  if (has(/^Mouse$/)) return "mouse";
  if (has(/^(Camera|Image)$/)) return "camera";
  if (has(/^(DiskDrive|CDROM)$/)) return "usbdrive";
  if (has(/^Net$/)) return "ethernet";
  if (has(/^Printer$/)) return "printer";
  return "usb";
}

const isCapture = (d) => up(d.id).includes("{0.0.1.");
function audioIcon(d) {
  if (isCapture(d)) return "mic";
  return /ヘッドホン|ヘッドセット|headphone|headset|buds|イヤホン/i.test(d.name)
    ? "headphones"
    : "speaker";
}

function fmtBytes(n) {
  if (!n) return "";
  const tb = n / 1e12;
  return tb >= 1
    ? `${tb.toFixed(tb >= 10 ? 0 : 1)} TB`
    : `${Math.round(n / 1e9)} GB`;
}
const netExtra = (d) =>
  (mapState.extra?.nets || []).find((x) => up(x.id) === up(d.id));
const diskExtra = (d) =>
  (mapState.extra?.disks || []).find((x) => up(x.id) === up(d.id));

function searchText() {
  return document.getElementById("mText").value.trim().toLowerCase();
}
function matches(d, extra = "") {
  const q = searchText();
  return (
    !q ||
    [mapName(d), d.name, d.bus_name, d.manufacturer, extra].some((s) =>
      String(s || "")
        .toLowerCase()
        .includes(q),
    )
  );
}

// ---------- 仕分け ----------
function categorize() {
  const all = mapState.ov.devices;
  const showVirtual = document.getElementById("mVirtual").checked;
  const cats = {
    screen: { title: "画面", icon: "monitor", items: [], virtual: [] },
    audio: { title: "音", icon: "sound", items: [], virtual: [] },
    input: { title: "入力", icon: "input", items: [], virtual: [] },
    bt: { title: "Bluetooth", icon: "bluetooth", items: [], virtual: [] },
    net: { title: "ネットワーク", icon: "ethernet", items: [], virtual: [] },
    store: { title: "保存", icon: "ssd", items: [], virtual: [] },
    other: { title: "その他", icon: "other", items: [], virtual: [] },
  };
  const used = new Set();
  const add = (cat, d, item, virtualOverride) => {
    used.add(up(d.id));
    if (!matches(d, item.matchText)) return;
    const v = virtualOverride ?? isVirtualDev(d);
    (v ? cats[cat].virtual : cats[cat].items).push({ dev: d, ...item });
  };

  // USB は木で描くので、ここでは「使った」印だけ
  for (const d of all) if (up(d.id).startsWith("USB\\")) used.add(up(d.id));

  // 画面: GPU (モニターの絵は別に描く)
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

  // 音: 鳴らす側と録る側 (並べ方と印は audioCardHtml で)
  for (const d of all.filter((x) => x.class === "AudioEndpoint")) {
    add("audio", d, { icon: audioIcon(d), capture: isCapture(d) });
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
    const top = topDevice(d) || d;
    const k = up(top.id);
    if (!inputs.has(k)) inputs.set(k, { top, kinds: new Set() });
    inputs.get(k).kinds.add(kind);
  }
  for (const { top, kinds } of inputs.values()) {
    const ks = [...kinds];
    const u = up(top.id);
    const icon = u.startsWith("USB\\")
      ? guessUsbIcon(top)
      : ks.includes("ゲームパッド")
        ? "gamepad"
        : ks.length === 1 && ks[0] === "マウス"
          ? "mouse"
          : "keyboard";
    const via = u.startsWith("BTH")
      ? "Bluetooth"
      : u.startsWith("USB\\")
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

  // ネットワーク (本物かどうかは Windows の「物理的なアダプター」の印を優先)
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
    add(
      "net",
      d,
      { icon: wifi ? "wifi" : "ethernet", sub: st },
      e ? !e.physical : undefined,
    );
  }

  // 保存
  for (const d of all.filter(
    (x) => x.class === "DiskDrive" || x.class === "CDROM",
  )) {
    const e = diskExtra(d);
    const viaUsb = [...ancestors(d)].some((a) => up(a.id).startsWith("USB\\"));
    add("store", d, {
      icon: /nvme|ssd/i.test(d.id + d.name) ? "ssd" : "hdd",
      sub: [
        fmtBytes(e?.size),
        viaUsb ? "USB" : /NVME/.test(up(d.id)) ? "NVMe" : e?.iface,
      ]
        .filter(Boolean)
        .join("・"),
    });
  }

  // その他: シリアル (COM)・カメラ・プリンターなど
  for (const d of all.filter((x) => x.class === "Ports"))
    add("other", d, { icon: "serial", sub: "シリアル（COM）" });
  for (const d of all) {
    if (used.has(up(d.id))) continue;
    const u = up(d.id);
    if (/^(BTHLEDEVICE|BTHENUM|BTH|HID)\\/.test(u)) continue;
    if (!d.class || INTERNAL_CLASSES.has(d.class)) continue;
    const icon = /Camera|Image/.test(d.class)
      ? "camera"
      : /Printer/.test(d.class)
        ? "printer"
        : "chip";
    add("other", d, { icon, sub: d.class });
  }

  const internal = all.filter((d) => !used.has(up(d.id)) && matches(d));
  return { cats, internal, showVirtual };
}

// ---------- 描く ----------
function devRow(it) {
  const d = it.dev;
  const st = d.status !== "ok" ? statusBadge(d) : "";
  return `<button class="mrow ${it.flap ? "flap" : ""}" data-open="${esc(d.id)}" title="${esc(d.name)}">
    <span class="mic-wrap">${iconSvg(it.icon, 30)}</span>
    <span class="mtext"><span class="mname">${esc(mapName(d))}</span>${it.sub ? `<span class="sub">${esc(it.sub)}</span>` : ""}${it.flap ? `<span class="flapnote">くり返し外れています</span>` : ""}${it.extraHtml || ""}</span>
    ${st}
  </button>`;
}

function vnote(c, showVirtual) {
  return !showVirtual && c.virtual.length
    ? `<div class="sub mvnote">仮想の機器 ${c.virtual.length} 個（上の「仮想の機器も出す」で表示）</div>`
    : "";
}

function cardHtml(key, c, showVirtual, extraTop = "", countText = null) {
  const list = c.items.concat(showVirtual ? c.virtual : []);
  return `<section class="mcard" data-card="${key}" style="grid-area:${key}">
    <h3><span class="mic-wrap big">${iconSvg(c.icon, 34)}</span>${esc(c.title)}<span class="mcount">${countText ?? `${c.items.length} 台`}</span></h3>
    ${extraTop}
    <div class="mlist">${list.map(devRow).join("") || `<div class="sub">ありません</div>`}</div>
    ${vnote(c, showVirtual)}
  </section>`;
}

function monitorsHtml() {
  const ms = mapState.ov.monitors.filter((m) => m.width > 0);
  if (!ms.length) return `<div class="sub">画面の情報を取れませんでした</div>`;
  const minX = Math.min(...ms.map((m) => m.x)),
    minY = Math.min(...ms.map((m) => m.y));
  const W = Math.max(...ms.map((m) => m.x + m.width)) - minX;
  const H = Math.max(...ms.map((m) => m.y + m.height)) - minY;
  const HH = H * 1.22; // 台の分の余白
  const gpuName = (id) => {
    const g = mapDev(id);
    return g ? mapName(g) : "";
  };
  const mons = ms.map((m) => {
    const mon = mapDev(m.monitor_id);
    const nm =
      (mon && state.names.devices[up(mon.id)]) ||
      m.name ||
      (mon ? mon.name : "画面");
    return `<button class="mon" style="left:${((m.x - minX) / W) * 100}%;top:${((m.y - minY) / HH) * 100}%;width:${(m.width / W) * 100}%;height:${(m.height / HH) * 100}%" ${mon ? `data-open="${esc(mon.id)}"` : ""}>
      <span class="mon-screen">
        <span class="mon-name">${esc(nm)}</span>
        <span class="mon-spec">${m.width}×${m.height}・${Math.round(m.hz)}Hz</span>
        <span class="mon-spec">${esc(CONN[m.connection] ?? "不明なつなぎ方")}${m.primary ? "・<b>メイン</b>" : ""}</span>
        <span class="mon-gpu">${esc(gpuName(m.adapter_id))}</span>
      </span>
    </button>`;
  });
  return `<div class="mons-wrap"><div class="mons" style="aspect-ratio:${W} / ${HH};width:min(100%, ${Math.round((300 * W) / HH)}px)">${mons.join("")}</div></div>
    <p class="sub">Windows の画面設定と同じ並び方です。モニターを押すと、くわしく見られます。</p>`;
}

// ---------- USB の木 ----------
let usbHiddenHubs = 0;

function hasPluggedBelow(d) {
  return descendants(d).some(isPluggedUsb);
}

function usbNodeHtml(d, depth) {
  const row = state.devices.find((x) => up(x.id) === up(d.id));
  const flap = row && isFlap(row);
  const hub = isUsbHub(d);
  const port =
    d.port != null ? `<span class="uport">ポート${d.port}</span>` : "";
  const kids =
    hub && depth < 10
      ? kidsOf(d)
          .filter(
            (c) =>
              up(c.id).startsWith("USB\\") && (isUsbHub(c) || isPluggedUsb(c)),
          )
          .sort((a, b) => (a.port ?? 99) - (b.port ?? 99))
      : [];
  const childHtml = kids.map((c) => usbNodeHtml(c, depth + 1)).filter(Boolean);
  // 機器が1台もつながっていないハブは出さない (USB 3 のハブは USB 2 用と 2 つに見えるので、空のハブが多い)
  if (hub && !hasPluggedBelow(d)) {
    usbHiddenHubs++;
    return "";
  }
  const q = searchText();
  // 探しているときは、当たった機器と、そこまでのハブだけ出す
  if (q && !matches(d) && !childHtml.length) return "";
  const sub = hub
    ? isRootHub(d)
      ? "ルートハブ"
      : `ハブ・${kids.filter((c) => !isUsbHub(c)).length} 台`
    : "";
  const rowHtml = `<button class="mrow utree-row ${flap ? "flap" : ""} ${hub ? "is-hub" : ""}" data-open="${esc(d.id)}" title="${esc(d.name)}">
      <span class="mic-wrap">${iconSvg(guessUsbIcon(d), hub ? 26 : 30)}</span>
      <span class="mtext"><span class="mname">${port}${esc(mapName(d))}</span>${sub ? `<span class="sub">${esc(sub)}</span>` : ""}${flap ? `<span class="flapnote">くり返し外れています</span>` : ""}</span>
      ${d.status !== "ok" ? statusBadge(d) : ""}
    </button>`;
  return `<li>${rowHtml}${childHtml.length ? `<ul>${childHtml.join("")}</ul>` : ""}</li>`;
}

function usbCardHtml() {
  const all = mapState.ov.devices;
  const roots = all.filter(isRootHub);
  const byCtrl = new Map();
  for (const r of roots) {
    const k = up(r.parent);
    if (!byCtrl.has(k)) byCtrl.set(k, []);
    byCtrl.get(k).push(r);
  }
  const plugged = all.filter(isPluggedUsb).length;
  const hubs = all.filter(
    (d) => up(d.id).startsWith("USB\\") && d.is_hub && !isRootHub(d),
  ).length;
  usbHiddenHubs = 0;
  const groups = [...byCtrl.entries()]
    .map(([k, rs]) => {
      const ctrl = mapDev(k);
      const body = rs
        .map((r) => usbNodeHtml(r, 0))
        .filter(Boolean)
        .join("");
      if (!body) return "";
      return `<div class="ugroup"><div class="ugroup-title">${iconSvg("chip", 22)}<span>${esc(ctrl ? mapName(ctrl) : "USB コントローラー")}</span></div><ul class="utree">${body}</ul></div>`;
    })
    .filter(Boolean);
  return `<section class="mcard" data-card="usb" style="grid-area:usb">
    <h3><span class="mic-wrap big">${iconSvg("usb", 34)}</span>USB<span class="mcount">${plugged} 台・ハブ ${hubs} 個</span></h3>
    <p class="sub">PC の USB の口のまとまり（コントローラー）→ ハブ → 機器 の順です。</p>
    ${groups.join("") || `<div class="sub">ありません</div>`}
    ${usbHiddenHubs ? `<div class="sub mvnote">機器がつながっていないハブ ${usbHiddenHubs} 個は省きました（「つながり方」タブには全部出ます）。</div>` : ""}
  </section>`;
}

// ---------- 音 ----------
function audioRowExtra(d) {
  const a = mapState.audio;
  if (!a) return "";
  const u = up(d.id);
  const badges = [];
  if (u === a.render || u === a.capture)
    badges.push(`<span class="abadge main">いつも使う</span>`);
  if (u === a.render_comm || u === a.capture_comm)
    badges.push(`<span class="abadge comm">通話で使う</span>`);
  const ep = a.endpoints.find((e) => e.id === u);
  const names = appsOf(d);
  const apps = names.length
    ? `<span class="aapps">${isCapture(d) ? "録っているアプリ" : "鳴らしているアプリ"}：${esc(names.join("・"))}</span>`
    : "";
  const pct = Math.round(Math.min(1, ep?.peak || 0) * 100);
  return `${badges.length ? `<span class="abadges">${badges.join("")}</span>` : ""}
    <span class="ameter" data-ep="${esc(u)}" role="meter" aria-label="今の音の大きさ" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${pct}"><span style="width:${pct}%"></span></span>
    ${apps}`;
}

/** その機器で鳴らしている (録っている) アプリ。Windows の中のサービスは「Windows の音」にまとめる */
function appsOf(d) {
  const ep = mapState.audio?.endpoints.find((e) => e.id === up(d.id));
  const names = (ep?.apps || []).map((n) =>
    /^(svchost|audiodg)$/i.test(n) ? "Windows の音" : n,
  );
  return [...new Set(names)];
}

function audioRank(d) {
  const a = mapState.audio;
  if (!a) return 9;
  const u = up(d.id);
  if (u === a.render || u === a.capture) return 0;
  if (u === a.render_comm || u === a.capture_comm) return 1;
  const ep = a.endpoints.find((e) => e.id === u);
  if (appsOf(d).length || ep?.peak > 0) return 2;
  return 3;
}

function audioCardHtml(c, showVirtual) {
  // 既定の機器と、アプリが使っている機器は、仮想でも必ず出す (今使っている機器を見るため)
  const inUse = c.virtual
    .filter((it) => audioRank(it.dev) <= 1 || appsOf(it.dev).length)
    .map((it) => ({ ...it, sub: "仮想の機器" }));
  const rest = c.virtual.filter(
    (it) => !(audioRank(it.dev) <= 1 || appsOf(it.dev).length),
  );
  const list = c.items.concat(inUse, showVirtual ? rest : []);
  c = { ...c, virtual: rest };
  const part = (cap) =>
    list
      .filter((it) => !!it.capture === cap)
      .sort(
        (x, y) =>
          audioRank(x.dev) - audioRank(y.dev) ||
          mapName(x.dev).localeCompare(mapName(y.dev), "ja"),
      )
      .map((it) => devRow({ ...it, extraHtml: audioRowExtra(it.dev) }))
      .join("") || `<div class="sub">ありません</div>`;
  return `<section class="mcard" data-card="audio" style="grid-area:audio">
    <h3><span class="mic-wrap big">${iconSvg("sound", 34)}</span>音<span class="mcount">${c.items.length} 個</span></h3>
    <p class="sub">緑のバーが今の音の大きさです。「いつも使う」「通話で使う」は Windows の既定の機器です。</p>
    <h4 class="asub">鳴らす（スピーカー・ヘッドホン）</h4><div class="mlist">${part(false)}</div>
    <h4 class="asub">録る（マイク）</h4><div class="mlist">${part(true)}</div>
    ${vnote(c, showVirtual)}
  </section>`;
}

/** 音の大きさのバーだけを書き換える (全体を描き直さない) */
function updateMeters() {
  const a = mapState.audio;
  if (!a) return;
  for (const el of document.querySelectorAll(".ameter[data-ep]")) {
    const ep = a.endpoints.find((e) => e.id === el.dataset.ep);
    const pct = Math.round(Math.min(1, ep?.peak || 0) * 100);
    el.firstElementChild.style.width = pct + "%";
    el.setAttribute("aria-valuenow", pct);
  }
}

async function pollAudio() {
  try {
    const a = await invoke("get_audio_state");
    mapState.audio = a;
    // 既定の機器や鳴らしているアプリが変わったときだけ、箱を描き直す
    const sig = JSON.stringify([
      a.render,
      a.render_comm,
      a.capture,
      a.capture_comm,
      a.endpoints.map((e) => e.id + ":" + e.apps.join(",")),
    ]);
    if (sig !== mapState.audioSig) {
      mapState.audioSig = sig;
      renderMap();
    } else updateMeters();
  } catch {
    /* 音の情報が取れなくても、全体図は出す */
  }
}

function pcHtml() {
  const pc = mapState.ov.pc;
  const gpus = mapState.ov.devices.filter(
    (d) => d.class === "Display" && !isVirtualDev(d),
  );
  const spec = [
    pc.cpu
      ? [
          "cpu",
          pc.cpu.replace(/\s+/g, " "),
          pc.threads ? `${pc.threads} スレッド` : "",
        ]
      : null,
    pc.memory_gb
      ? ["memory", `メモリ ${Math.round(pc.memory_gb)} GB`, ""]
      : null,
    ...gpus.map((g) => ["gpu", mapName(g), "GPU"]),
  ].filter(Boolean);
  return `<section class="mcard mpc" data-card="pc" style="grid-area:pc">
    <div class="pc-pic">${iconSvg("pc", 96)}</div>
    <h3 class="pc-title">この PC</h3>
    <div class="pc-spec">${spec.map(([ic, a, b]) => `<div class="pc-line">${iconSvg(ic, 26)}<span><b>${esc(a)}</b>${b ? `<span class="sub"> ${esc(b)}</span>` : ""}</span></div>`).join("")}</div>
    <button class="pc-more" data-goto="spec">くわしいスペックを見る</button>
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
  // 開いていた「Windows の中の部品」のたたみを覚えておく
  const open = new Set(
    [
      ...document.querySelectorAll(
        '#map [data-card="internal"] details[open] summary',
      ),
    ].map((s) => s.textContent),
  );
  document.getElementById("map").innerHTML =
    `<svg class="mlines" id="mlines" aria-hidden="true"></svg>
    ${pcHtml()}
    ${cardHtml("screen", cats.screen, showVirtual, monitorsHtml(), `${mapState.ov.monitors.length} 台`)}
    ${usbCardHtml()}
    ${audioCardHtml(cats.audio, showVirtual)}
    ${cardHtml("input", cats.input, showVirtual)}
    ${cardHtml("bt", cats.bt, showVirtual)}
    ${cardHtml("net", cats.net, showVirtual)}
    ${cardHtml("store", cats.store, showVirtual)}
    ${cardHtml("other", cats.other, showVirtual)}
    ${internalHtml}`;
  for (const s of document.querySelectorAll(
    '#map [data-card="internal"] details summary',
  ))
    if (open.has(s.textContent)) s.parentElement.open = true;
  document.getElementById("mInfo").textContent =
    `全部で ${mapState.ov.devices.length} 個の機器（${fmtTime(Date.now())} に読みました）`;
  requestAnimationFrame(drawLines);
}

/** この PC から隣の箱へ線を引く (横に並ぶ広さがあるときだけ) */
function drawLines() {
  const map = document.getElementById("map");
  const svg = document.getElementById("mlines");
  const pc = map?.querySelector('[data-card="pc"]');
  if (!svg || !pc) return;
  const base = map.getBoundingClientRect();
  svg.setAttribute("width", base.width);
  svg.setAttribute("height", base.height);
  if (getComputedStyle(map).getPropertyValue("--narrow").trim() === "1") {
    svg.innerHTML = "";
    return;
  }
  const p = pc.getBoundingClientRect();
  const pcx = p.left + p.width / 2 - base.left,
    pcy = p.top + p.height / 2 - base.top;
  const paths = [];
  for (const card of map.querySelectorAll(".mcard:not(.mpc)")) {
    if (card.dataset.card === "internal" || card.dataset.card === "other")
      continue;
    const r = card.getBoundingClientRect();
    const cx = Math.min(
      Math.max(pcx, r.left - base.left + 24),
      r.right - base.left - 24,
    );
    const cy = Math.min(
      Math.max(pcy, r.top - base.top + 24),
      r.bottom - base.top - 24,
    );
    const horiz = Math.abs(cx - pcx) > Math.abs(cy - pcy);
    const x = horiz
      ? cx > pcx
        ? r.left - base.left
        : r.right - base.left
      : cx;
    const y = horiz ? cy : cy > pcy ? r.top - base.top : r.bottom - base.top;
    paths.push(
      `<path d="M${pcx} ${pcy} L${x} ${y}"/><circle cx="${x}" cy="${y}" r="5"/>`,
    );
  }
  svg.innerHTML = paths.join("");
}

async function loadOverviewData(withExtra) {
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
  if (withExtra || !mapState.extra) {
    invoke("get_overview_extra")
      .then((e) => {
        mapState.extra = e;
        if (mapVisible()) renderMap();
      })
      .catch(() => {});
  }
  return ov;
}

async function loadMap(withExtra) {
  if (mapState.loading) return;
  mapState.loading = true;
  try {
    await loadOverviewData(withExtra);
    renderMap();
    pollAudio();
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
// 全体図を見ている間だけ、音の大きさを 0.5 秒ごとに読む
setInterval(() => {
  if (mapVisible() && mapState.ov && !document.hidden) pollAudio();
}, 500);

document.getElementById("map").addEventListener("click", (e) => {
  const g = e.target.closest("[data-goto]");
  if (g) {
    document.querySelector(`[data-tab="${g.dataset.goto}"]`)?.click();
    return;
  }
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
