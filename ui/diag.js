// 「診断・お手入れ」タブ:
// - いつもと違う変化チェッカー (構成スナップショット比較)
// - スリープ解除・スリープ妨害の犯人診断
// - 予期せぬ再起動・ブルースクリーン・エラー履歴
// - Windows 自動起動アプリ整理
// - 使っていない昔の USB 機器 (ゴースト) のお掃除

const diagState = {
  ram: null,
  baseline: null,
  wake: null,
  events: null,
  startups: null,
  spec: null,
  ov: null,
  loading: false,
};

const fmtGb = (mb) => (mb / 1024).toFixed(1) + " GB";

function ramCardHtml() {
  const r = diagState.ram;
  const s = state.settings || {};
  const autoOn = !!s.auto_ram_boost;
  const thr = s.auto_ram_threshold_pct ?? 80;
  const iv = s.auto_ram_interval_min ?? 15;
  const pct = r ? r.used_pct : 0;
  const level = pct >= 85 ? "high" : pct >= 70 ? "mid" : "low";
  const lastTxt = r?.last_boost_time
    ? `最後の解放：${fmtTime(r.last_boost_time)}（${r.last_freed_mb} MB 解放・自動 ${r.auto_boost_count} 回）`
    : "まだ実行していません";
  return `<section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("memory", 32)}</span>RAM 解放・定期ブースト（Microsoft PC Manager 相当）</h3>
    <div id="ramLive">
      ${
        r
          ? `<div class="rate-head">
              <span>使用中 <b>${fmtGb(r.used_mb)}</b> ／ 合計 ${fmtGb(r.total_mb)}（空き ${fmtGb(r.avail_mb)}）</span>
              <span class="rate-level ${level}">使用率 ${pct}%</span>
            </div>
            <span class="sbar ${pct >= 85 ? "low" : ""}" style="height:10px;margin:6px 0"><span style="width:${pct}%"></span></span>
            <div class="sub">一時ファイル（%TEMP%）：約 ${r.temp_mb} MB　／　${esc(lastTxt)}</div>`
          : `<div class="sub">読んでいます…</div>`
      }
    </div>
    <p class="sub" style="margin-top:8px"><b>注意：</b>Windows は空いているメモリを、ファイルを速く読むための置き場（キャッシュ）に使っています。解放すると一時的に「空き」は増えますが、あとでアプリがメモリを取り戻すときに読み直しや、ページファイル（SSD）とのやり取りが増え、<b>ゲームがカクついたり SSD への書き込みが増えたりすることがあります</b>。メモリが本当に足りなくて困っているとき（使用率がずっと 90% 以上など）だけ使うのがおすすめです。</p>
    <div class="row-btns" style="margin-top:10px">
      <button class="primary" id="btnBoostRam">今すぐ RAM を解放</button>
      <button id="btnBoostAll">RAM ＋ 一時ファイルをまとめて解放</button>
    </div>
    <p class="sub">一時ファイルは、%TEMP% の中で <b>24 時間以上さわられていない物だけ</b>を消します（インストールの途中の物や、動いているアプリが使っている物を守るため）。</p>
    <div class="msgline" id="ramMsg"></div>
    <div class="section" style="margin-top:12px;padding-top:10px">
      <h4 style="margin-top:0">定期的な自動解放（スマートブースト）</h4>
      <div class="row-btns" style="gap:12px">
        <label><input type="checkbox" id="ramAutoOn" ${autoOn ? "checked" : ""} /> 定期的に自動で RAM を解放する</label>
        <label>条件：
          <select id="ramAutoThr">
            <option value="80" ${thr === 80 ? "selected" : ""}>使用率 80% 以上のとき</option>
            <option value="70" ${thr === 70 ? "selected" : ""}>使用率 70% 以上のとき</option>
            <option value="60" ${thr === 60 ? "selected" : ""}>使用率 60% 以上のとき</option>
            <option value="0" ${thr === 0 ? "selected" : ""}>使用率に関係なく毎回</option>
          </select>
        </label>
        <label>間隔：
          <select id="ramAutoIv">
            <option value="5" ${iv === 5 ? "selected" : ""}>5 分ごと</option>
            <option value="15" ${iv === 15 ? "selected" : ""}>15 分ごと</option>
            <option value="30" ${iv === 30 ? "selected" : ""}>30 分ごと</option>
            <option value="60" ${iv === 60 ? "selected" : ""}>60 分ごと</option>
          </select>
        </label>
      </div>
      <p class="sub" style="margin-top:6px">使われていないメモリ（ワーキングセット）を安全に解放します。管理者で開いているときは Windows のスタンバイキャッシュも合わせて整理します。設定タブで「トレイにしまう」をオンにしておくと、裏でずっと定期解放してくれます。</p>
      <div class="msgline" id="ramOptMsg"></div>
    </div>
  </section>`;
}

function buildSnapshot(ov, spec) {
  const usb = (ov?.devices || [])
    .filter((d) => up(d.id).startsWith("USB\\") && !up(d.id).includes("&MI_") && !d.is_hub)
    .map((d) => ({ id: up(d.id), name: state.names.devices[up(d.id)] || d.bus_name || d.name }))
    .sort((a, b) => a.id.localeCompare(b.id));
  const monitors = (ov?.monitors || [])
    .filter((m) => m.width > 0)
    .map((m) => ({
      name: m.name || m.gdi_name || "画面",
      width: m.width,
      height: m.height,
      hz: Math.round(m.hz),
      primary: !!m.primary,
    }));
  const mods = spec?.memory?.modules || [];
  const memSpeeds = mods.map((m) => m.configured || m.speed || 0).filter((v) => v > 0);
  const gpus = (spec?.gpu || [])
    .filter((g) => !/virtual|parsec|meta|idd|basic display/i.test(g.name || ""))
    .map((g) => ({ name: g.name || "", driver: g.driver || "" }));
  const disks = (spec?.disks || []).map((d) => ({ name: d.name || "", size: d.size || 0 }));
  return {
    saved_at: Date.now(),
    usb,
    monitors,
    memory: {
      total: spec?.memory?.total || 0,
      min_mhz: memSpeeds.length ? Math.min(...memSpeeds) : 0,
    },
    gpus,
    bios: spec?.bios?.version || "",
    disks,
  };
}

function diffSnapshots(base, cur) {
  const diffs = [];
  // モニター (リフレッシュレート低下・台数変化)
  for (const bm of base.monitors || []) {
    const cm = (cur.monitors || []).find((m) => m.name === bm.name);
    if (!cm) {
      diffs.push({ level: "warn", text: `モニター「${bm.name}」が見つかりません（以前は ${bm.width}×${bm.height}・${bm.hz}Hz）` });
    } else {
      if (bm.hz > 65 && cm.hz <= 65 && cm.hz < bm.hz) {
        diffs.push({ level: "bad", text: `モニター「${bm.name}」のリフレッシュレートが ${bm.hz}Hz → ${cm.hz}Hz に下がっています（60Hz に戻っている可能性があります）` });
      } else if (Math.abs(bm.hz - cm.hz) >= 5) {
        diffs.push({ level: "info", text: `モニター「${bm.name}」のリフレッシュレートが変わりました（${bm.hz}Hz → ${cm.hz}Hz）` });
      }
      if (bm.width !== cm.width || bm.height !== cm.height) {
        diffs.push({ level: "info", text: `モニター「${bm.name}」の解像度が変わりました（${bm.width}×${bm.height} → ${cm.width}×${cm.height}）` });
      }
    }
  }
  // メモリ速度・容量
  const bMem = base.memory || {};
  const cMem = cur.memory || {};
  if (bMem.min_mhz && cMem.min_mhz && cMem.min_mhz < bMem.min_mhz - 100) {
    diffs.push({ level: "bad", text: `メモリの速さが下がっています（${bMem.min_mhz} MHz → ${cMem.min_mhz} MHz。BIOS の XMP / EXPO がオフになった可能性があります）` });
  }
  if (bMem.total && cMem.total && Math.abs(bMem.total - cMem.total) > 512 * 1024 * 1024) {
    const gb = (v) => Math.round(v / 1024 ** 3);
    diffs.push({ level: "bad", text: `認識されているメモリ容量が変わりました（${gb(bMem.total)} GB → ${gb(cMem.total)} GB）` });
  }
  // USB 機器の欠け・増加
  const curUsbIds = new Set((cur.usb || []).map((u) => u.id));
  const baseUsbIds = new Set((base.usb || []).map((u) => u.id));
  for (const bu of base.usb || []) {
    if (!curUsbIds.has(bu.id)) {
      diffs.push({ level: "warn", text: `いつもつながっている USB 機器が見つかりません：${bu.name}` });
    }
  }
  for (const cu of cur.usb || []) {
    if (!baseUsbIds.has(cu.id)) {
      diffs.push({ level: "info", text: `新しくつながっている USB 機器：${cu.name}` });
    }
  }
  // GPU ドライバー・BIOS
  for (const bg of base.gpus || []) {
    const cg = (cur.gpus || []).find((g) => g.name === bg.name);
    if (cg && bg.driver && cg.driver && bg.driver !== cg.driver) {
      diffs.push({ level: "info", text: `GPU「${bg.name}」のドライバーが変わりました（${bg.driver} → ${cg.driver}）` });
    }
  }
  if (base.bios && cur.bios && base.bios !== cur.bios) {
    diffs.push({ level: "info", text: `BIOS のバージョンが変わりました（${base.bios} → ${cur.bios}）` });
  }
  // ディスクの欠け
  for (const bd of base.disks || []) {
    if (!(cur.disks || []).some((d) => d.name === bd.name)) {
      diffs.push({ level: "bad", text: `ディスク「${bd.name}」が見つかりません` });
    }
  }
  return diffs;
}

function baselineCardHtml() {
  const b = diagState.baseline;
  let body = "";
  if (!b) {
    body = `<p class="sub">まだ基準が保存されていません。PC が正常に動いているときに下のボタンを押して保存しておくと、あとから「60Hz に戻った」「XMP が切れた」「USB 機器が1つ認識されていない」などを自動で見つけられます。</p>`;
  } else if (!diagState.ov || !diagState.spec) {
    body = `<p class="sub">今の状態と比べています…</p>`;
  } else {
    const cur = buildSnapshot(diagState.ov, diagState.spec);
    const diffs = diffSnapshots(b, cur);
    const saved = new Date(b.saved_at).toLocaleString("ja-JP");
    body = `<p class="sub">基準を保存した日時：${esc(saved)}（USB ${(b.usb || []).length} 台・画面 ${(b.monitors || []).length} 台）</p>`;
    if (!diffs.length) {
      body += `<div class="diag-ok">基準のときと変わりありません（問題なし）</div>`;
    } else {
      body += `<div class="diag-list">${diffs
        .map((d) => `<div class="diag-item ${esc(d.level)}">${esc(d.text)}</div>`)
        .join("")}</div>`;
    }
  }
  return `<section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("pc", 32)}</span>いつもと違う変化チェッカー</h3>
    <div class="row-btns" style="margin-bottom:8px">
      <button class="primary" id="btnSaveBase">今の状態を基準として保存</button>
      <button id="btnCheckBase">今すぐ比べ直す</button>
    </div>
    <div class="msgline" id="baseMsg"></div>
    ${body}
  </section>`;
}

function wakeCardHtml() {
  const w = diagState.wake;
  if (!w) {
    return `<section class="scard"><h3><span class="mic-wrap big">${iconSvg("other", 32)}</span>スリープ解除・妨害の診断</h3><p class="sub">調べています…</p></section>`;
  }
  const armed = w.wake_armed.length
    ? `<ul class="diag-ul">${w.wake_armed
        .map(
          (name) =>
            `<li><span>${esc(name)}</span><button class="small-btn" data-wake-off="${esc(name)}" ${state.admin ? "" : 'title="管理者の権限が必要です"'}>起こせないようにする</button></li>`,
        )
        .join("")}</ul>`
    : `<p class="sub">スリープ解除を許可されている機器はありません。</p>`;
  return `<section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("other", 32)}</span>スリープ解除・妨害の診断</h3>
    <p class="sub">夜中に PC が勝手に起きる・画面が消えない原因を調べます。${state.admin ? "" : "（「起こせないようにする」や詳しい妨害一覧には管理者の権限が要ります）"}</p>
    <h4>最後に PC を起こした原因</h4>
    <pre class="diag-pre">${esc(w.last_wake || "情報なし")}</pre>
    <h4>スリープ解除を許可されている機器（マウス・LAN など）</h4>
    ${armed}
    <div class="msgline" id="wakeMsg"></div>
    <h4>今スリープを止めているアプリ・機器</h4>
    <pre class="diag-pre">${esc(w.requests || "情報なし")}</pre>
  </section>`;
}

function sysEventsCardHtml() {
  const evs = diagState.events;
  const kindWord = {
    power: "電源落ち",
    bsod: "強制終了",
    gpu: "画面リセット",
    usb: "USB エラー",
    disk: "ディスク警告",
  };
  let body = `<p class="sub">読んでいます…</p>`;
  if (evs) {
    if (!evs.length) {
      body = `<div class="diag-ok">過去 30 日間に、急な電源落ち・ブルースクリーン・画面や USB の大きなエラーはありませんでした。</div>`;
    } else {
      body = `<div class="diag-list">${evs
        .slice(0, 25)
        .map(
          (e) => `<div class="diag-item ${e.kind === "power" || e.kind === "bsod" ? "bad" : "warn"}">
            <div class="diag-ev-head"><span class="t">${esc(new Date(e.time).toLocaleString("ja-JP"))}</span><span class="badge ${e.kind === "power" || e.kind === "bsod" ? "problem" : "disabled"}">${esc(kindWord[e.kind] || e.kind)}</span><b>${esc(e.title)}</b></div>
            <div class="sub">${esc(e.message)}</div>
          </div>`,
        )
        .join("")}</div>`;
    }
  }
  return `<section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("chip", 32)}</span>急な電源落ち・ブルースクリーン等の履歴（過去 30 日）</h3>
    <p class="sub">Windows の記録から、電源落ち（Kernel-Power 41）・ブルースクリーン・GPU ドライバー落ち・USB/ディスク警告を抜き出しています。</p>
    ${body}
  </section>`;
}

function startupCardHtml() {
  const list = diagState.startups;
  let body = `<p class="sub">読んでいます…</p>`;
  const scopeLabel = { hkcu: "自分のみ", hklm: "PC 全体", folder: "フォルダー" };
  if (list) {
    body = list.length
      ? `<div class="diag-list">${list
          .map(
            (s) => `<div class="startup-row">
              <div class="startup-main">
                <b>${esc(s.name)}</b>
                <span class="badge ${s.enabled ? "ok" : "absent"}">${s.enabled ? "オン" : "オフ"}</span>
                <span class="sub">${esc(scopeLabel[s.scope] || s.scope)}</span>
                <div class="sub" style="word-break:break-all">${esc(s.command)}</div>
              </div>
              <div>${
                s.can_toggle
                  ? `<button class="small-btn" data-st-scope="${esc(s.scope)}" data-st-name="${esc(s.name)}" data-st-en="${s.enabled ? "0" : "1"}">${s.enabled ? "オフにする" : "オンに戻す"}</button>`
                  : `<span class="sub">${s.scope === "hklm" ? "管理者が必要" : "フォルダー管理"}</span>`
              }</div>
            </div>`,
          )
          .join("")}</div>`
      : `<p class="sub">自動起動の項目は見つかりませんでした。</p>`;
  }
  return `<section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("pc", 32)}</span>Windows 起動時に自動で立ち上がるアプリ</h3>
    <p class="sub">タスク マネージャーと同じ仕組みでオン・オフするので、オフにしても後からすぐ戻せます。</p>
    <div class="msgline" id="startupMsg"></div>
    ${body}
  </section>`;
}

function ghostCardHtml() {
  const ghosts = state.devices
    .filter((d) => !d.present && !d.is_interface && up(d.id).startsWith("USB\\"))
    .sort((a, b) => displayName(a).localeCompare(displayName(b), "ja"));
  let body = "";
  if (!ghosts.length) {
    body = `<div class="diag-ok">使っていない昔の USB 機器の残骸はありません。</div>`;
  } else {
    body = `<div class="row-btns" style="margin-bottom:8px">
      <button id="ghostSelectAll">すべて選ぶ（${ghosts.length} 件）</button>
      <button class="danger" id="ghostDelete" ${state.admin ? "" : "disabled"}>選んだ古い機器を削除</button>
      ${state.admin ? "" : `<span class="sub">削除には「管理者で開き直す」が必要です</span>`}
    </div>
    <div class="msgline" id="ghostMsg"></div>
    <div class="ghost-list">${ghosts
      .map(
        (d) => `<label class="ghost-row">
          <input type="checkbox" class="ghost-chk" value="${esc(d.id)}" />
          <span><b>${esc(displayName(d))}</b> <span class="sub">${d.last_removal ? `最後に外れた：${esc(new Date(d.last_removal).toLocaleDateString("ja-JP"))}` : ""} / ${esc(d.id)}</span></span>
        </label>`,
      )
      .join("")}</div>`;
  }
  return `<section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("usb", 32)}</span>使っていない昔の USB 機器のお掃除</h3>
    <p class="sub">今つながっていない過去の USB 機器の記録をまとめて消せます（現在つながっている機器は安全のため表示しません）。消しても、またさせば自動で元どおり認識されます。</p>
    ${body}
  </section>`;
}

function renderDiag() {
  const el = document.getElementById("diag");
  if (!el) return;
  el.innerHTML =
    ramCardHtml() +
    baselineCardHtml() +
    wakeCardHtml() +
    sysEventsCardHtml() +
    startupCardHtml() +
    ghostCardHtml();
}

async function refreshRam(withTemp = false) {
  try {
    const r = await invoke("get_ram_status", { withTemp });
    if (!withTemp && diagState.ram) r.temp_mb = diagState.ram.temp_mb;
    diagState.ram = r;
    updateHeaderRam(r);
    const box = document.getElementById("ramLive");
    if (box) {
      const pct = r.used_pct;
      const level = pct >= 85 ? "high" : pct >= 70 ? "mid" : "low";
      const lastTxt = r.last_boost_time
        ? `最後の解放：${fmtTime(r.last_boost_time)}（${r.last_freed_mb} MB 解放・自動 ${r.auto_boost_count} 回）`
        : "まだ実行していません";
      box.innerHTML = `<div class="rate-head">
          <span>使用中 <b>${fmtGb(r.used_mb)}</b> ／ 合計 ${fmtGb(r.total_mb)}（空き ${fmtGb(r.avail_mb)}）</span>
          <span class="rate-level ${level}">使用率 ${pct}%</span>
        </div>
        <span class="sbar ${pct >= 85 ? "low" : ""}" style="height:10px;margin:6px 0"><span style="width:${pct}%"></span></span>
        <div class="sub">一時ファイル（%TEMP%）：約 ${r.temp_mb} MB　／　${esc(lastTxt)}</div>`;
    }
  } catch {}
}

async function loadDiag() {
  if (diagState.loading) return;
  diagState.loading = true;
  renderDiag();
  try {
    const [ram, base, wake, events, startups, ov, spec] = await Promise.all([
      invoke("get_ram_status", { withTemp: true }).catch(() => null),
      invoke("get_baseline").catch(() => null),
      invoke("get_wake_info").catch(() => null),
      invoke("get_system_events").catch(() => []),
      invoke("get_startup_apps").catch(() => []),
      mapState.ov ? Promise.resolve(mapState.ov) : invoke("get_overview").catch(() => null),
      specState.data ? Promise.resolve(specState.data) : invoke("get_spec").catch(() => null),
    ]);
    diagState.ram = ram;
    if (ram) updateHeaderRam(ram);
    diagState.baseline = base;
    diagState.wake = wake;
    diagState.events = events;
    diagState.startups = startups;
    diagState.ov = ov;
    diagState.spec = spec;
  } finally {
    diagState.loading = false;
  }
  renderDiag();
}

const diagVisible = () => !document.getElementById("tab-diag").hidden;
setInterval(() => {
  if (!document.hidden) refreshRam(false);
}, 4000);
listen("ram-boosted", () => refreshRam(false));

document.getElementById("diagReload").onclick = () => {
  diagState.ov = null;
  diagState.spec = null;
  loadDiag();
};

document.getElementById("diag").addEventListener("change", async (e) => {
  if (["ramAutoOn", "ramAutoThr", "ramAutoIv"].includes(e.target.id)) {
    const msg = document.getElementById("ramOptMsg");
    try {
      state.settings = await invoke("save_app_settings", {
        settings: {
          ...state.settings,
          auto_ram_boost: document.getElementById("ramAutoOn").checked,
          auto_ram_threshold_pct: Number(document.getElementById("ramAutoThr").value),
          auto_ram_interval_min: Number(document.getElementById("ramAutoIv").value),
        },
      });
      setMsg(
        msg,
        state.settings.auto_ram_boost
          ? "定期自動解放をオンにしました"
          : "定期自動解放をオフにしました",
        "good",
      );
    } catch (err) {
      setMsg(msg, String(err), "err");
    }
  }
});

document.getElementById("diag").addEventListener("click", async (e) => {
  if (e.target.id === "btnBoostRam" || e.target.id === "btnBoostAll") {
    const cleanTemp = e.target.id === "btnBoostAll";
    const msg = document.getElementById("ramMsg");
    setMsg(msg, cleanTemp ? "RAM と一時ファイルを解放しています…" : "RAM を解放しています…");
    try {
      const r = await invoke("boost_ram", { cleanTemp });
      await refreshRam(cleanTemp);
      const extra = [];
      if (r.purged_standby) extra.push("スタンバイキャッシュ整理済み");
      if (cleanTemp) extra.push(`一時ファイル ${r.temp_freed_mb} MB 削除`);
      setMsg(
        msg,
        `${r.freed_mb} MB 解放しました（使用率 ${r.before_pct}% → ${r.after_pct}%・対象 ${r.trimmed_procs} プロセス${extra.length ? "・" + extra.join("・") : ""}）`,
        "good",
      );
    } catch (err) {
      setMsg(msg, String(err), "err");
    }
    return;
  }
  if (e.target.id === "btnSaveBase") {
    const msg = document.getElementById("baseMsg");
    setMsg(msg, "今の状態を調べて保存しています…");
    try {
      const [ov, spec] = await Promise.all([invoke("get_overview"), invoke("get_spec")]);
      diagState.ov = ov;
      diagState.spec = spec;
      const snap = buildSnapshot(ov, spec);
      await invoke("save_baseline", { snapshot: snap });
      diagState.baseline = snap;
      renderDiag();
      setMsg(document.getElementById("baseMsg"), "今の状態を基準として保存しました", "good");
    } catch (err) {
      setMsg(msg, String(err), "err");
    }
    return;
  }
  if (e.target.id === "btnCheckBase") {
    const msg = document.getElementById("baseMsg");
    setMsg(msg, "比べ直しています…");
    try {
      const [ov, spec] = await Promise.all([invoke("get_overview"), invoke("get_spec")]);
      diagState.ov = ov;
      diagState.spec = spec;
      renderDiag();
      setMsg(document.getElementById("baseMsg"), "最新の状態と比べました", "good");
    } catch (err) {
      setMsg(msg, String(err), "err");
    }
    return;
  }
  const wakeBtn = e.target.closest("[data-wake-off]");
  if (wakeBtn) {
    const msg = document.getElementById("wakeMsg");
    const name = wakeBtn.dataset.wakeOff;
    setMsg(msg, "設定を変えています…");
    try {
      await invoke("disable_device_wake", { deviceName: name });
      diagState.wake = await invoke("get_wake_info");
      renderDiag();
      setMsg(document.getElementById("wakeMsg"), `「${name}」でスリープ解除しないようにしました`, "good");
    } catch (err) {
      setMsg(msg, String(err), "err");
    }
    return;
  }
  const stBtn = e.target.closest("[data-st-name]");
  if (stBtn) {
    const msg = document.getElementById("startupMsg");
    const scope = stBtn.dataset.stScope;
    const name = stBtn.dataset.stName;
    const enabled = stBtn.dataset.stEn === "1";
    setMsg(msg, "変えています…");
    try {
      await invoke("set_startup_enabled", { scope, name, enabled });
      diagState.startups = await invoke("get_startup_apps");
      renderDiag();
      setMsg(document.getElementById("startupMsg"), `「${name}」を${enabled ? "オン" : "オフ"}にしました`, "good");
    } catch (err) {
      setMsg(msg, String(err), "err");
    }
    return;
  }
  if (e.target.id === "ghostSelectAll") {
    const boxes = [...document.querySelectorAll(".ghost-chk")];
    const allChecked = boxes.every((b) => b.checked);
    boxes.forEach((b) => (b.checked = !allChecked));
    return;
  }
  if (e.target.id === "ghostDelete") {
    const ids = [...document.querySelectorAll(".ghost-chk:checked")].map((b) => b.value);
    const msg = document.getElementById("ghostMsg");
    if (!ids.length) {
      setMsg(msg, "消したい機器にチェックを入れてください", "err");
      return;
    }
    if (!confirm(`選んだ ${ids.length} 件の古い USB 機器の記録を Windows から削除しますか？\n（またさせば自動で認識し直します）`)) {
      return;
    }
    setMsg(msg, `削除しています（${ids.length} 件）…`);
    let ok = 0;
    const errs = [];
    for (const id of ids) {
      try {
        await invoke("remove_ghost_device", { id });
        ok++;
      } catch (err) {
        errs.push(String(err));
      }
    }
    await refresh();
    renderDiag();
    const m2 = document.getElementById("ghostMsg");
    if (errs.length) setMsg(m2, `${ok} 件削除しました（失敗 ${errs.length} 件：${errs[0]}）`, "err");
    else setMsg(m2, `${ok} 件の古い記録を削除しました`, "good");
  }
});
