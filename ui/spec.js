// PC のスペックタブ。map.js の mapState・loadOverviewData・mapName・isVirtualDev・CONN・fmtBytes も使う。

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
const specState = { data: null, loading: false };

const gb = (n) => (n ? `${Math.round(n / 1024 ** 3)} GB` : "");
const gbExact = (n) => (n ? `${(n / 1024 ** 3).toFixed(1)} GB` : "");

function uptimeText(iso) {
  if (!iso) return "";
  const ms = Date.now() - new Date(iso).getTime();
  const h = Math.floor(ms / 3600000),
    d = Math.floor(h / 24);
  return d
    ? `${d} 日 ${h % 24} 時間`
    : `${h} 時間 ${Math.floor((ms % 3600000) / 60000)} 分`;
}

/** 表示用の行: [見出し, 中身, 補足] の配列を作る。コピーする文字にも使う */
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
      ["名前", c.name],
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
      ["合計", gb(m.total), types],
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
      ],
      [
        "BIOS",
        [s.bios?.maker, s.bios?.version].filter(Boolean).join(" "),
        s.bios?.date,
      ],
    ],
  });

  const gpus = (s.gpu || []).filter((g) => !/^ROOT\\/i.test(g.pnp || ""));
  const vgpus = (s.gpu || []).length - gpus.length;
  out.push({
    key: "gpu",
    icon: "gpu",
    title: "GPU",
    rows: [
      ...gpus.flatMap((g) => [
        ["名前", g.name],
        ["ビデオメモリ", g.vram ? gbExact(g.vram) : ""],
        ["ドライバー", g.driver, g.driver_date],
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
      ]),
      ...(s.volumes || []).map((v) => [
        `${v.letter}: ドライブ`,
        `空き ${fmtBytes(v.free)} / ${fmtBytes(v.size)}`,
        v.fs,
        v.size ? v.free / v.size : null,
      ]),
    ],
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
      ]),
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
  for (const sec of out) sec.rows = sec.rows.filter((r) => r[1]);
  return out.filter((sec) => sec.rows.length);
}

function renderSpec() {
  const el = document.getElementById("spec");
  if (!specState.data) return;
  el.innerHTML = specSections()
    .map(
      (sec) => `<section class="scard">
    <h3><span class="mic-wrap big">${iconSvg(sec.icon, 32)}</span>${esc(sec.title)}</h3>
    <dl class="skv">${sec.rows
      .map(
        ([k, v, note, ratio]) =>
          `<dt>${esc(k)}</dt><dd><b>${esc(v)}</b>${note ? `<span class="sub">　${esc(note)}</span>` : ""}${
            ratio != null
              ? `<span class="sbar ${ratio < 0.1 ? "low" : ""}" role="meter" aria-label="空きの割合" aria-valuenow="${Math.round(ratio * 100)}" aria-valuemin="0" aria-valuemax="100"><span style="width:${Math.round(ratio * 100)}%"></span></span>`
              : ""
          }</dd>`,
      )
      .join("")}</dl>
  </section>`,
    )
    .join("");
}

function specText() {
  return specSections()
    .map(
      (sec) =>
        `【${sec.title}】\n` +
        sec.rows
          .map(([k, v, note]) => `${k}：${v}${note ? `（${note}）` : ""}`)
          .join("\n"),
    )
    .join("\n\n");
}

async function loadSpec(force) {
  if (specState.loading) return;
  specState.loading = true;
  const info = document.getElementById("sInfo");
  info.textContent = "読んでいます…（数秒かかります）";
  try {
    const [s] = await Promise.all([
      invoke("get_spec"),
      mapState.ov && !force ? Promise.resolve() : loadOverviewData(false),
      invoke("get_audio_state")
        .then((a) => (mapState.audio = a))
        .catch(() => {}),
    ]);
    specState.data = s;
    renderSpec();
    info.textContent = `${fmtTime(Date.now())} に読みました`;
  } catch (e) {
    info.textContent = "読めませんでした: " + e;
  } finally {
    specState.loading = false;
  }
}

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
