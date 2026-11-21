// PC のスペックタブ。map.js の mapState・loadOverviewData・mapDev・mapName・CONN・fmtBytes・isPluggedUsb も使う。
// アップグレードの提案は、部品の名前を押したときだけ、その部品の箱の中に出す (まずスペックを見てもらうため)。

const MEM_TYPE = {
  20: "DDR",
  21: "DDR2",
  24: "DDR3",
  26: "DDR4",
  27: "LPDDR",
  28: "LPDDR2",
  29: "LPDDR3",
  30: "LPDDR4",
  34: "DDR5",
  35: "LPDDR5",
};
const specState = { data: null, usage: null, loading: false, open: new Set() };

/** CPU・GPU の発売時期の目安 (上から順に当てる)。このアプリを作った 2026 年時点の情報 */
const RELEASES = [
  // CPU (AMD)
  [/Ryzen \d+ 9\d{3}X3D/i, "2024-11"],
  [/Ryzen \d+ 9\d{3}/i, "2024-08"],
  [/Ryzen \d+ 7\d{3}X3D/i, "2023-04"],
  [/Ryzen \d+ 7\d{3}/i, "2022-09"],
  [/Ryzen 7 5700X3D/i, "2024-01"],
  [/Ryzen \d+ 5\d{3}X3D/i, "2022-04"],
  [/Ryzen \d+ 5\d{3}/i, "2020-11"],
  [/Ryzen \d+ 3\d{3}/i, "2019-07"],
  [/Ryzen \d+ 2\d{3}/i, "2018-04"],
  // CPU (Intel)
  [/Core\(?T?M?\)? Ultra \d+ 2\d{2}/i, "2024-10"],
  [/i\d-14\d{3}/i, "2023-10"],
  [/i\d-13\d{3}/i, "2022-10"],
  [/i\d-12\d{3}/i, "2021-11"],
  [/i\d-11\d{3}/i, "2021-03"],
  [/i\d-10\d{3}/i, "2020-05"],
  [/i\d-9\d{3}/i, "2018-10"],
  // GPU (AMD)
  [/RX 9070/i, "2025-03"],
  [/RX 9060/i, "2025-06"],
  [/RX 7900/i, "2022-12"],
  [/RX 7[78]00/i, "2023-09"],
  [/RX 7600/i, "2023-05"],
  [/RX 6[89]\d0/i, "2020-11"],
  [/RX 6750/i, "2022-05"],
  [/RX 6700/i, "2021-03"],
  [/RX 6600/i, "2021-10"],
  [/RX 5[67]00/i, "2019-07"],
  // GPU (NVIDIA)
  [/RTX 50[89]0/i, "2025-01"],
  [/RTX 5070/i, "2025-02"],
  [/RTX 5060/i, "2025-04"],
  [/RTX 4090/i, "2022-10"],
  [/RTX 4080/i, "2022-11"],
  [/RTX 4070/i, "2023-04"],
  [/RTX 4060/i, "2023-06"],
  [/RTX 30[89]0/i, "2020-09"],
  [/RTX 3070/i, "2020-10"],
  [/RTX 3060/i, "2021-02"],
  [/RTX 20\d0/i, "2018-09"],
  [/GTX 16\d0/i, "2019-04"],
];

const releaseOf = (name) =>
  RELEASES.find(([re]) => re.test(name || ""))?.[1] || null;

const gb = (n) => (n ? `${Math.round(n / 1024 ** 3)} GB` : "");
const gbExact = (n) => (n ? `${(n / 1024 ** 3).toFixed(1)} GB` : "");

/** 時間 (h) を「1年2か月」のように */
function fmtSpan(hours) {
  if (hours == null) return "";
  const days = hours / 24;
  if (days < 1) return `${Math.round(hours)} 時間`;
  if (days < 31) return `${Math.floor(days)} 日`;
  const months = Math.floor(days / 30.44);
  const y = Math.floor(months / 12);
  const m = months % 12;
  if (!y) return `${m}か月`;
  return m ? `${y}年${m}か月` : `${y}年`;
}
const hoursSince = (date) => (Date.now() - new Date(date).getTime()) / 3600000;
const fmtHours = (h) => `${Math.round(h).toLocaleString("ja-JP")} 時間`;

function uptimeText(iso) {
  if (!iso) return "";
  const ms = Date.now() - new Date(iso).getTime();
  const h = Math.floor(ms / 3600000);
  const d = Math.floor(h / 24);
  return d
    ? `${d} 日 ${h % 24} 時間`
    : `${h} 時間 ${Math.floor((ms % 3600000) / 60000)} 分`;
}

const physicalGpus = () =>
  (specState.data?.gpu || []).filter((g) => !/^ROOT\\/i.test(g.pnp || ""));
const badUsb = () =>
  (state.devices || []).filter((d) => d.present && d.status === "problem");

// ---------- アップグレードの提案 ----------
const LEVEL = {
  now: { label: "すぐやるとよい", cls: "now" },
  maybe: { label: "余裕があれば", cls: "maybe" },
  ok: { label: "今のままで十分", cls: "ok" },
};

/** 提案の一覧。key はどの部品の箱に出すか (disk / memory / cpu / gpu / board / screen / usb / net) */
function suggestions() {
  const s = specState.data;
  if (!s) return [];
  const out = [];
  const add = (key, x) => out.push({ key, ...x });
  const board = [s.board?.maker, s.board?.product].filter(Boolean).join(" ");
  let pcie = "";
  if (/X870|B850|X670|B650/i.test(board)) pcie = "PCIe 5.0";
  else if (/X570|B550|Z[4-8]90|B[4-8]60/i.test(board)) pcie = "PCIe 4.0";
  const mons = mapState.ov?.monitors || [];
  const maxHz = Math.max(0, ...mons.map((m) => m.hz));
  const maxW = Math.max(0, ...mons.map((m) => m.width));

  // ディスクの空き
  for (const v of s.volumes || []) {
    if (!v.size) continue;
    const r = v.free / v.size;
    if (r >= 0.2) continue;
    add("disk", {
      level: r < 0.1 ? "now" : "maybe",
      title: `${v.letter}: ドライブの空きが少ない（残り ${Math.round(r * 100)}%・${fmtBytes(v.free)}）`,
      why: "空きが1〜2割を切ると、Windows やゲームの更新が失敗したり、動きが遅くなったりします。",
      what: `まずは「設定 → システム → ストレージ」で要らないファイルを消してください。それでも足りなければ、2TB ほどの NVMe SSD を足すのがおすすめです${pcie ? `（このマザーボードは ${pcie} の M.2 に対応）` : ""}。空いている M.2 の口があるかは、マザーボードの説明書で確かめてください。`,
    });
  }

  // ディスクの健康 (健康記録が取れたとき)
  const unsafeDisks = [];
  for (const d of s.disks || []) {
    if (d.health && d.health !== "Healthy")
      add("disk", {
        level: "now",
        title: `${d.name} の状態が「${d.health}」`,
        why: "Windows がディスクの不調を見つけています。",
        what: "大事なファイルを今すぐ別の場所にコピーして、ディスクの交換を考えてください。",
      });
    if (d.critical)
      add("disk", {
        level: "now",
        title: `${d.name} が警告を出している`,
        why: "SSD 自身が、温度・予備の領域・読み書きの異常などの警告を記録しています。",
        what: "大事なファイルを今すぐ別の場所にコピーして、交換を考えてください。",
      });
    if (d.media_errors)
      add("disk", {
        level: "now",
        title: `${d.name} に読み書きの失敗が ${d.media_errors} 回`,
        why: "データが正しく読み書きできなかった記録があります。",
        what: "バックアップを取って、様子を見ながら交換を考えてください。",
      });
    if (d.wear != null && d.wear >= 80)
      add("disk", {
        level: "now",
        title: `${d.name} の消耗が ${d.wear}%`,
        why: "SSD は書き込める量に上限があり、100% に近づくと壊れやすくなります。",
        what: "バックアップを取って、新しい SSD への交換を考えてください。",
      });
    else if (d.wear != null && d.wear >= 50) {
      const perDay = d.written && d.hours ? d.written / (d.hours / 24) : 0;
      add("disk", {
        level: "maybe",
        title: `${d.name} の消耗が ${d.wear}%（寿命の半分を使った）`,
        why: `SSD は書き込める量に上限があります。今までに ${fmtBytes(d.written) || "多く"} 書き込んでいます${perDay ? `（電源が入っていた日で割ると 1 日あたり約 ${Math.round(perDay / 1e9)} GB）` : ""}。すぐ壊れるわけではありませんが、このまま使うと何年かで交換の時期が来ます。`,
        what: "大事なファイルは別の場所にもコピーしておき、次に SSD を買うときの候補にしてください。録画・動画の編集先・ゲームの置き場など、書き込みが多い物を別の SSD に分けると長持ちします。何が書き込んでいるかは、Windows の「リソース モニター」の「ディスク」タブで見られます。",
      });
    }
    if (d.temp != null && d.temp >= 70)
      add("disk", {
        level: "maybe",
        title: `${d.name} が熱い（${d.temp}℃）`,
        why: "熱いと速さが落ちたり、寿命が縮んだりします。",
        what: "M.2 用のヒートシンクを付けるか、ケースの風の流れを見直してください。",
      });
    if (d.unsafe && d.starts >= 20 && d.unsafe / d.starts >= 0.5)
      unsafeDisks.push(`${d.name}（${d.unsafe} / ${d.starts} 回）`);
  }
  if (unsafeDisks.length)
    add("disk", {
      level: "maybe",
      title: `正しい手順で止まらなかった回数が多い：${unsafeDisks.join("・")}`,
      why: "停電・電源ボタンの長押し・コンセントを抜くなどで増えます。Windows の高速スタートアップや休止状態でも数えられることがあり、必ず問題があるとは限りません。",
      what: "PC を止めるときは「スタート → 電源 → シャットダウン」を使ってください。停電が多い場所なら UPS（無停電電源装置）も考えてください。",
    });

  // メモリ
  const m = s.memory || {};
  const mods = m.modules || [];
  const totalGb = m.total / 1024 ** 3;
  const free = m.slots ? m.slots - mods.length : 0;
  if (totalGb < 15) {
    add("memory", {
      level: "now",
      title: `メモリが少ない（${gb(m.total)}）`,
      why: "今のゲームやブラウザーは 16 GB 以上あると安心です。",
      what: `同じ種類（${[...new Set(mods.map((x) => MEM_TYPE[x.kind]).filter(Boolean))].join("・") || "今と同じ規格"}）のメモリを足して、16〜32 GB にしてください。`,
    });
  } else if (mods.length === 1) {
    add("memory", {
      level: "maybe",
      title: "メモリが1枚だけ",
      why: "メモリは2枚1組で使うと速くなります（デュアルチャネル）。",
      what: "同じ容量・同じ速さのメモリをもう1枚足してください。",
    });
  } else if (totalGb < 31) {
    add("memory", {
      level: "maybe",
      title: `メモリ ${gb(m.total)}`,
      why: "ゲームをしながら配信や通話をすると、足りなくなることがあります。",
      what: `32 GB にすると余裕が出ます${free ? `（空いている口が ${free} つ）` : ""}。`,
    });
  } else {
    const part = mods[0]?.part;
    add("memory", {
      level: "ok",
      title: `メモリ ${gb(m.total)}`,
      why: "ゲーム・配信・通話を同時にしても足りる量です。",
      what:
        free && part
          ? `増やすなら、同じ型番（${part}）を空いている ${free} つの口に足すのが安全です。`
          : "",
    });
  }
  // XMP / EXPO が切れていないか (型番に書いてある速さより遅く動いている)
  const first = mods[0];
  if (first) {
    const rated = Number(
      (first.part || "").match(/(2[4-9]|3\d|[4-8]\d)00/)?.[0],
    );
    const now = first.configured || first.speed;
    if (rated && now && now + 100 < rated)
      add("memory", {
        level: "now",
        title: `メモリが本来より遅く動いている（${now} MT/s、型番では ${rated}）`,
        why: "BIOS の XMP（AMD では EXPO / DOCP）が切れていると、メモリが遅い設定のまま動きます。",
        what: "BIOS で XMP / EXPO を有効にしてください。お金はかかりません。",
      });
  }

  // CPU
  const cpu = (s.cpu || [])[0];
  if (cpu) {
    const am4 = /AM4/i.test(cpu.socket || "");
    const x3d = /X3D/i.test(cpu.name);
    const short = cpu.name.replace(/ \d+-Core Processor/i, "");
    if (am4 && !x3d) {
      add("cpu", {
        level: maxHz >= 144 ? "maybe" : "ok",
        title: `CPU（${short}）`,
        why:
          maxHz >= 144
            ? `${Math.round(maxHz)}Hz の画面を使っているので、ゲームの fps を上げると効果が見えやすいです。`
            : "普段使いやゲームには十分な速さです。",
        what: "マザーボード（AM4）とメモリをそのまま使える一番の強化は、Ryzen 7 5700X3D / 5800X3D への交換です（ゲームの fps が上がりやすい CPU）。交換の前に BIOS の更新が要ることがあります。もっと上を狙うなら AM5（Ryzen 7 9800X3D など）ですが、マザーボードと DDR5 メモリも買い替えになります。",
      });
    } else if (am4 && x3d) {
      add("cpu", {
        level: "ok",
        title: `CPU（${short}）`,
        why: "AM4 では一番ゲームに強い部類の CPU です。",
        what: "これ以上は AM5 への乗り換え（マザーボード・メモリも交換）になります。",
      });
    } else {
      const rel = releaseOf(cpu.name);
      if (rel && hoursSince(rel) / 8766 >= 6)
        add("cpu", {
          level: "maybe",
          title: `CPU が発売から約 ${fmtSpan(hoursSince(rel))}`,
          why: "新しいゲームやアプリで重く感じることがあります。",
          what: "CPU だけでなく、マザーボードとメモリも一緒に新しくすることが多いです。",
        });
      else
        add("cpu", {
          level: "ok",
          title: `CPU（${short}）`,
          why: "今のところ十分な速さです。",
          what: "",
        });
    }
  }

  // GPU
  for (const g of physicalGpus()) {
    const vramGb = (g.vram || 0) / 1024 ** 3;
    const rel = releaseOf(g.name);
    const age = rel ? hoursSince(rel) / 8766 : 0;
    if (vramGb && vramGb < 7.5) {
      add("gpu", {
        level: "now",
        title: `GPU のビデオメモリが少ない（${gbExact(g.vram)}）`,
        why: "今のゲームは 8 GB 以上を求めることが多く、足りないと画質を下げてもカクつきます。",
        what: "12〜16 GB の GPU（RX 9060 XT 16GB / RTX 5060 Ti 16GB クラス）がおすすめです。",
      });
    } else if (maxW > 1920 || (maxHz >= 240 && age >= 4)) {
      add("gpu", {
        level: "maybe",
        title: `GPU（${g.name}${rel ? `・発売から約 ${fmtSpan(hoursSince(rel))}` : ""}）`,
        why:
          maxW > 1920
            ? `${maxW} 幅の画面は 1080p より重く、GPU の力が要ります。`
            : `${Math.round(maxHz)}Hz の画面を、重いゲームで出し切るには力が足りないことがあります。`,
        what: "RX 9070 XT / RTX 5070 Ti クラスにすると、高い fps を保ちやすくなります。電源の容量（750W 以上が目安）とケースの長さも確かめてください。",
      });
    } else {
      add("gpu", {
        level: "ok",
        title: `GPU（${g.name}）`,
        why: `今の画面（${maxW || "?"} 幅・${Math.round(maxHz)}Hz）なら十分な力です。`,
        what: "",
      });
    }
    if (g.driver_date && hoursSince(g.driver_date) / 24 > 365)
      add("gpu", {
        level: "maybe",
        title: `GPU のドライバーが古い（${g.driver_date}）`,
        why: "新しいゲームの不具合直しや速さの改善が入っていないことがあります。",
        what: "AMD / NVIDIA のアプリから最新のドライバーを入れてください。",
      });
  }

  // BIOS
  if (s.bios?.date && hoursSince(s.bios.date) / 8766 >= 3) {
    add("board", {
      level: "maybe",
      title: `BIOS が古い（${s.bios.date}・${s.bios.version || ""}）`,
      why: "BIOS の更新には、不具合や安全の問題の直し、新しい CPU への対応が入っています。",
      what: `${board ? `${board} の` : "マザーボードの"}メーカーのページから最新の BIOS を入れてください。更新中に電源が切れると起動しなくなることがあるので、時間に余裕があるときにしてください。`,
    });
  }

  // モニターの使用時間
  for (const mo of mons) {
    const u = (specState.usage || []).find((x) => x.gdi_name === mo.gdi_name);
    if (u?.usage_hours >= 30000)
      add("screen", {
        level: "maybe",
        title: `${mo.name} を長く使っている（${fmtHours(u.usage_hours)}）`,
        why: "画面の明るさ（バックライト）は 3〜5万時間ほどで暗くなってくると言われます。",
        what: "暗く感じたら、明るさを上げるか買い替えを考えてください。",
      });
    else if (u?.usage_hours)
      add("screen", {
        level: "ok",
        title: `${mo.name}（${fmtSpan(u.usage_hours)}使用）`,
        why: "バックライトが暗くなってくる目安（3〜5万時間）まで、まだ余裕があります。",
        what: "",
      });
  }

  // USB の不調
  const bad = badUsb();
  if (bad.length)
    add("usb", {
      level: "maybe",
      title: `うまく動いていない USB 機器が ${bad.length} 台`,
      why: "コード43 などのエラーは、ケーブル・差し込み口・機器の故障・電力不足が原因のことが多いです。",
      what: "「一覧」タブで場所を確かめて、ケーブルや差し込み口を替えてみてください。「抜いて当てる」で、どの機器かを探せます。",
    });

  // ネットワーク
  for (const n of s.nets || []) {
    const mbps =
      Number.parseFloat(n.speed) * (/Gbps/i.test(n.speed) ? 1000 : 1);
    if (n.status === "Up" && mbps && mbps < 1000)
      add("net", {
        level: "maybe",
        title: `有線 LAN が ${n.speed} でつながっている`,
        why: "この LAN は 1 Gbps 以上に対応していることが多く、ケーブルが古いと遅くなります。",
        what: "カテゴリ 6（CAT6）以上の LAN ケーブルに替えてみてください。",
      });
    else if (n.status === "Up")
      add("net", {
        level: "ok",
        title: `${n.name}（${n.speed}）`,
        why: "家のネットの速さを出し切れる速さです。",
        what: "",
      });
  }

  const order = { now: 0, maybe: 1, ok: 2 };
  return out.sort((a, b) => order[a.level] - order[b.level]);
}

// ---------- 表に出す中身 ----------
/** 行は [見出し, 中身, 補足, 割合(バー), 提案の key]。提案の key がある行は、中身を押すとその部品の提案が開く */
function specSections() {
  const s = specState.data;
  const out = [];
  if (!s) return out;
  const os = s.os || {};
  out.push({
    key: "os",
    icon: "pc",
    title: "OS",
    rows: [
      ["名前", os.name, os.arch],
      [
        "版",
        [os.version, os.build && `ビルド ${os.build}`]
          .filter(Boolean)
          .join("・"),
      ],
      ["起動してから", uptimeText(os.boot)],
    ],
  });

  out.push({
    key: "cpu",
    icon: "cpu",
    title: "CPU",
    rows: (s.cpu || []).flatMap((c) => [
      ["名前", c.name, "", null, "cpu"],
      ["コア・スレッド", `${c.cores} コア / ${c.threads} スレッド`],
      ["基本の速さ", c.mhz ? `${(c.mhz / 1000).toFixed(1)} GHz` : ""],
      [
        "キャッシュ",
        [c.l2 && `L2 ${c.l2 / 1024} MB`, c.l3 && `L3 ${c.l3 / 1024} MB`]
          .filter(Boolean)
          .join("・"),
      ],
      ["ソケット", c.socket],
    ]),
  });

  const m = s.memory || {};
  const mods = m.modules || [];
  const types = [
    ...new Set(mods.map((x) => MEM_TYPE[x.kind]).filter(Boolean)),
  ].join("・");
  out.push({
    key: "memory",
    icon: "memory",
    title: "メモリ",
    rows: [
      ["合計", gb(m.total), types, null, "memory"],
      [
        "差してある数",
        m.slots
          ? `${mods.length} 枚（差し込み口 ${m.slots} 個）`
          : `${mods.length} 枚`,
      ],
      ...mods.map((x, i) => [
        `${i + 1} 枚目`,
        [
          gb(x.size),
          MEM_TYPE[x.kind],
          (x.configured || x.speed) && `${x.configured || x.speed} MT/s`,
        ]
          .filter(Boolean)
          .join("・"),
        [x.maker && x.maker !== "Unknown" ? x.maker : "", x.part]
          .filter(Boolean)
          .join(" "),
        null,
        "memory",
      ]),
    ],
  });

  out.push({
    key: "board",
    icon: "chip",
    title: "マザーボード・BIOS",
    rows: [
      [
        "マザーボード",
        [s.board?.maker, s.board?.product].filter(Boolean).join(" "),
        "",
        null,
        "board",
      ],
      [
        "BIOS",
        [s.bios?.maker, s.bios?.version].filter(Boolean).join(" "),
        s.bios?.date,
        null,
        "board",
      ],
    ],
  });

  const gpus = physicalGpus();
  const vgpus = (s.gpu || []).length - gpus.length;
  out.push({
    key: "gpu",
    icon: "gpu",
    title: "GPU",
    rows: [
      ...gpus.flatMap((g) => [
        ["名前", g.name, "", null, "gpu"],
        ["ビデオメモリ", g.vram ? gbExact(g.vram) : ""],
        ["ドライバー", g.driver, g.driver_date, null, "gpu"],
      ]),
      ...(vgpus
        ? [
            [
              "仮想の画面アダプター",
              `${vgpus} 個`,
              "Parsec などのアプリが作った物",
            ],
          ]
        : []),
    ],
  });

  out.push({
    key: "disk",
    icon: "ssd",
    title: "ディスク",
    rows: [
      ...(s.disks || []).map((d) => [
        d.name,
        [fmtBytes(d.size), d.media !== "Unspecified" ? d.media : "", d.bus]
          .filter(Boolean)
          .join("・"),
        d.health === "Healthy" ? "正常" : d.health,
        null,
        "disk",
      ]),
      ...(s.volumes || []).map((v) => [
        `${v.letter}: ドライブ`,
        `空き ${fmtBytes(v.free)} / ${fmtBytes(v.size)}`,
        v.fs,
        v.size ? v.free / v.size : null,
        "disk",
      ]),
    ],
  });

  out.push({
    key: "health",
    icon: "ssd",
    title: "ストレージの寿命",
    rows: storageLifeRows(s),
  });

  out.push({
    key: "net",
    icon: "ethernet",
    title: "ネットワーク",
    rows: (s.nets || []).map((n) => [
      n.name,
      n.status === "Up"
        ? `つながっている・${n.speed}`
        : n.status === "Disconnected"
          ? "つながっていない"
          : n.status,
      "",
      null,
      "net",
    ]),
  });

  if (mapState.ov) {
    out.push({
      key: "screen",
      icon: "monitor",
      title: "画面",
      rows: mapState.ov.monitors.map((x) => [
        x.name || "画面",
        `${x.width}×${x.height}・${Math.round(x.hz)}Hz`,
        [CONN[x.connection], x.primary ? "メイン" : ""]
          .filter(Boolean)
          .join("・"),
        null,
        "screen",
      ]),
    });
    const plugged = mapState.ov.devices.filter(isPluggedUsb).length;
    const bad = badUsb().length;
    out.push({
      key: "usb",
      icon: "usb",
      title: "USB",
      rows: [
        [
          "つながっている機器",
          `${plugged} 台`,
          "くわしくは「全体図」と「一覧」タブ",
        ],
        [
          "うまく動いていない機器",
          bad ? `${bad} 台` : "なし",
          "",
          null,
          bad ? "usb" : null,
        ],
      ],
    });
  }
  const a = mapState.audio;
  if (a && mapState.ov) {
    const nm = (id) => {
      const d = mapDev(id);
      return d ? mapName(d) : "";
    };
    out.push({
      key: "audio",
      icon: "sound",
      title: "音（今使っている機器）",
      rows: [
        [
          "鳴らす",
          nm(a.render),
          a.render_comm && a.render_comm !== a.render
            ? `通話は ${nm(a.render_comm)}`
            : "通話も同じ",
        ],
        [
          "録る",
          nm(a.capture),
          a.capture_comm && a.capture_comm !== a.capture
            ? `通話は ${nm(a.capture_comm)}`
            : "通話も同じ",
        ],
      ],
    });
  }

  // 使った時間・年数
  const life = [];
  for (const mo of mapState.ov?.monitors || []) {
    const u = (specState.usage || []).find((x) => x.gdi_name === mo.gdi_name);
    if (u?.usage_hours) {
      const br =
        u.brightness != null
          ? `・今の明るさ ${Math.round((u.brightness / (u.brightness_max || 100)) * 100)}%`
          : "";
      life.push([
        mo.name,
        `画面がついていた時間 ${fmtSpan(u.usage_hours)}`,
        fmtHours(u.usage_hours) + br,
        null,
        "screen",
      ]);
    } else if (specState.usage)
      life.push([
        mo.name,
        "記録なし",
        "このモニターは使用時間を PC に教えていません",
      ]);
  }
  const smart = (s.disks || []).filter((d) => d.hours != null);
  for (const d of smart) {
    const notes = [
      fmtHours(d.hours),
      d.temp != null && `${d.temp}℃`,
      d.wear != null && `消耗 ${d.wear}%`,
      d.starts && `電源を入れた回数 ${d.starts.toLocaleString("ja-JP")} 回`,
      d.written && `書き込んだ量 ${fmtBytes(d.written)}`,
      d.unsafe &&
        `正しい手順で止まらなかった回数 ${d.unsafe.toLocaleString("ja-JP")} 回`,
    ];
    life.push([
      d.name,
      `電源が入っていた時間 ${fmtSpan(d.hours)}`,
      notes.filter(Boolean).join("・"),
      null,
      "disk",
    ]);
  }
  if (!smart.length && (s.disks || []).length)
    life.push([
      "ディスク",
      "管理者で開くと見られます",
      "電源が入っていた時間・温度・消耗度は、管理者の権限が要ります",
    ]);
  const cpuRel = releaseOf((s.cpu || [])[0]?.name);
  if (cpuRel)
    life.push([
      "CPU の発売から",
      `約 ${fmtSpan(hoursSince(cpuRel))}`,
      `${cpuRel.replace("-", " 年 ")} 月ごろ発売（目安）`,
    ]);
  for (const g of gpus) {
    const r = releaseOf(g.name);
    if (r)
      life.push([
        "GPU の発売から",
        `約 ${fmtSpan(hoursSince(r))}`,
        `${r.replace("-", " 年 ")} 月ごろ発売（目安）`,
      ]);
  }
  if (os.install)
    life.push([
      "今の Windows を入れてから",
      fmtSpan(hoursSince(os.install)),
      `${os.install.slice(0, 10)}（大きな更新を入れると、この日付も変わります）`,
    ]);
  if (s.bios?.date)
    life.push([
      "BIOS が作られてから",
      fmtSpan(hoursSince(s.bios.date)),
      s.bios.date,
    ]);
  out.push({
    key: "life",
    icon: "other",
    title: "使った時間・年数",
    rows: life,
  });

  for (const sec of out) sec.rows = sec.rows.filter((r) => r[1]);
  return out.filter((sec) => sec.rows.length);
}

/** ディスクの健康状態 (CrystalDiskInfo に近い分け方)。文字と色の両方で出す */
function diskHealth(d) {
  if (
    (d.health && d.health !== "Healthy") ||
    d.critical ||
    d.media_errors ||
    (d.wear != null && d.wear >= 90)
  )
    return "危険";
  if ((d.wear != null && d.wear >= 80) || (d.temp != null && d.temp >= 70))
    return "注意";
  return "良好";
}

/** ストレージの寿命の行。消耗度がある SSD は残りの割合とあと何年か、HDD は時間と状態で出す */
function storageLifeRows(s) {
  const disks = s.disks || [];
  const rows = [];
  const haveData = disks.some((d) => d.hours != null || d.wear != null);
  if (!haveData) {
    rows.push([
      "寿命",
      "管理者で開くと見られます",
      "SSD・HDD 自身の記録（SMART）を読むには、管理者の権限が要ります",
      null,
      state.admin ? null : "admin",
    ]);
    return rows;
  }
  for (const d of disks) {
    const hdd = /HDD/i.test(d.media || "");
    const label = diskHealth(d);
    if (d.wear != null && !hdd) {
      const left = Math.max(0, 100 - d.wear);
      let note = "";
      if (d.wear >= 1 && d.hours) {
        // 今までの書き込みの速さのまま使った場合 (電源が入っている時間で数える)
        const leftHours = d.hours * (left / d.wear);
        note = `今までと同じ使い方なら、あと約 ${fmtSpan(leftHours)}（電源が入っている時間で数えた目安）`;
      } else if (d.wear < 1) note = "まだほとんど減っていません";
      rows.push([
        d.name,
        `${label}・寿命の残り ${left}%`,
        note,
        left / 100,
        "disk",
      ]);
    } else {
      rows.push([
        d.name,
        `${label}`,
        hdd
          ? "HDD には消耗度がありません。状態と電源が入っていた時間で見ます"
          : "このディスクは消耗度を教えていません",
        null,
        "disk",
      ]);
    }
    const detail = [
      d.hours != null && `電源 ${fmtSpan(d.hours)}（${fmtHours(d.hours)}）`,
      d.temp != null && `温度 ${d.temp}℃`,
      d.written && `書き込んだ量 ${fmtBytes(d.written)}`,
      d.written &&
        d.hours &&
        `1日あたり約 ${Math.round(d.written / (d.hours / 24) / 1e9)} GB`,
      d.media_errors != null && `読み書きの失敗 ${d.media_errors} 回`,
    ]
      .filter(Boolean)
      .join("・");
    if (detail) rows.push(["", detail]);
  }
  return rows;
}

/** 部品の名前ではない見出し (この行は中身のほうを押せるようにする) */
const GENERIC_LABEL =
  /^(名前|合計|\d+ 枚目|マザーボード|BIOS|ドライバー|うまく動いていない機器|[A-Z]: ドライブ)$/;

function sugHtml(list) {
  return list
    .map(
      (x) => `<div class="sug ${LEVEL[x.level].cls}">
      <div class="sug-head"><span class="sug-level">${LEVEL[x.level].label}</span><b>${esc(x.title)}</b></div>
      <div class="sug-why">${esc(x.why)}</div>
      ${x.what ? `<div class="sug-what">${esc(x.what)}</div>` : ""}
    </div>`,
    )
    .join("");
}

function renderSpec() {
  const el = document.getElementById("spec");
  if (!specState.data) return;
  const sug = suggestions();
  const hasSug = (key) => sug.some((x) => x.key === key);
  el.innerHTML = specSections()
    .map((sec) => {
      const rows = sec.rows
        .map(([k, v, note, ratio, sk]) => {
          // 見出しが部品の名前の行 (ディスク名・モニター名など) は見出しを、「名前」「合計」などの行は中身を押せるようにする
          const link = (text) =>
            `<button class="slink" data-sug="${sk}" aria-expanded="${specState.open.has(sec.key + "|" + sk)}">${esc(text)}</button>`;
          const clickable = sk && sk !== "admin" && hasSug(sk);
          const nameInLabel = clickable && !GENERIC_LABEL.test(k);
          const label = nameInLabel ? link(k) : esc(k);
          let val = clickable && !nameInLabel ? link(v) : `<b>${esc(v)}</b>`;
          if (sk === "admin")
            val += ` <button class="primary" data-admin>管理者で開き直す</button>`;
          const bar =
            ratio != null
              ? `<span class="sbar ${ratio < 0.1 ? "low" : ""}" role="meter" aria-label="空きの割合" aria-valuenow="${Math.round(ratio * 100)}" aria-valuemin="0" aria-valuemax="100"><span style="width:${Math.round(ratio * 100)}%"></span></span>`
              : "";
          return `<dt>${label}</dt><dd>${val}${note ? `<span class="sub">　${esc(note)}</span>` : ""}${bar}</dd>`;
        })
        .join("");
      const opened = [...specState.open]
        .filter((o) => o.startsWith(sec.key + "|"))
        .map((o) => o.split("|")[1]);
      const panel = opened.length
        ? `<div class="sug-panel"><h4>アップグレードの提案</h4>${sugHtml(sug.filter((x) => opened.includes(x.key)))}<p class="sub">製品名はこのアプリを作った時点（2026年）の目安です。値段と在庫はお店で確かめてください。</p></div>`
        : "";
      return `<section class="scard" data-sec="${sec.key}">
      <h3><span class="mic-wrap big">${iconSvg(sec.icon, 32)}</span>${esc(sec.title)}</h3>
      <dl class="skv">${rows}</dl>${panel}
    </section>`;
    })
    .join("");
}

function specText() {
  const body = specSections()
    .map(
      (sec) =>
        `【${sec.title}】\n` +
        sec.rows
          .map(([k, v, note]) => `${k}：${v}${note ? `（${note}）` : ""}`)
          .join("\n"),
    )
    .join("\n\n");
  const sug = suggestions().filter((x) => x.level !== "ok");
  const tail = sug.length
    ? `\n\n【アップグレードの提案（目安）】\n${sug.map((x) => `・${LEVEL[x.level].label}：${x.title}`).join("\n")}`
    : "";
  return body + tail;
}

async function loadSpec(force) {
  if (specState.loading) return;
  specState.loading = true;
  const info = document.getElementById("sInfo");
  info.textContent = "読んでいます…（数秒かかります）";
  try {
    const [s, usage, nvme] = await Promise.all([
      invoke("get_spec"),
      invoke("get_monitor_usage").catch(() => null),
      invoke("get_nvme_health").catch(() => []),
      mapState.ov && !force ? Promise.resolve() : loadOverviewData(false),
      invoke("get_audio_state")
        .then((a) => (mapState.audio = a))
        .catch(() => {}),
    ]);
    // NVMe の健康記録 (管理者のときだけ取れる) を、同じ番号のディスクに混ぜる
    for (const d of s.disks || []) {
      const h = (nvme || []).find((x) => x.number === d.number);
      if (!h) continue;
      Object.assign(d, {
        hours: h.power_on_hours,
        temp: h.temp_c || d.temp,
        wear: h.percent_used,
        starts: h.power_cycles,
        written: h.written_bytes,
        unsafe: h.unsafe_shutdowns,
        media_errors: h.media_errors,
        critical: h.critical_warning,
      });
    }
    specState.data = s;
    specState.usage = usage;
    renderSpec();
    info.textContent = `${fmtTime(Date.now())} に読みました${state.admin ? "" : "（ディスクの使用時間は、管理者で開くと見られます）"}`;
  } catch (e) {
    info.textContent = "読めませんでした: " + e;
  } finally {
    specState.loading = false;
  }
}

document.getElementById("spec").addEventListener("click", (e) => {
  if (e.target.closest("[data-admin]")) {
    invoke("relaunch_admin").catch((err) => alert(String(err)));
    return;
  }
  const b = e.target.closest("[data-sug]");
  if (!b) return;
  const id = b.closest("[data-sec]").dataset.sec + "|" + b.dataset.sug;
  if (specState.open.has(id)) specState.open.delete(id);
  else specState.open.add(id);
  renderSpec();
});
document
  .getElementById("sReload")
  .addEventListener("click", () => loadSpec(true));
document.getElementById("sCopy").addEventListener("click", async () => {
  const msg = document.getElementById("sCopyMsg");
  const text = specText();
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
  setMsg(
    msg,
    "コピーしました。Discord などに貼り付けられます（PC の名前やシリアル番号は入っていません）",
    "good",
  );
});
