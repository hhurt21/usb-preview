// 「SSD の書き込み」タブ。今の書き込みの速さ、ブラウザーのキャッシュの設定、SSD を長持ちさせる説明。
// index.html の state・invoke・esc・setMsg と、icons.js の iconSvg を使う。

const ssdState = { rates: [], procRates: [], history: new Map(), policy: [], timer: null };

const mbps = (b) => b / 1024 / 1024;
const perDayGb = (bps) => (bps * 86400) / 1024 ** 3;
const fmtMb = (b) => (b >= 1024 ** 3 ? `${(b / 1024 ** 3).toFixed(2)} GB` : `${(b / 1024 / 1024).toFixed(1)} MB`);

function procRateRows() {
  if (!ssdState.procRates.length) return `<div class="sub">計測しています（2秒ごとに更新します）…</div>`;
  const maxRate = Math.max(1024 * 1024, ...ssdState.procRates.map((p) => p.write_bytes_per_sec));
  return ssdState.procRates
    .map((p) => {
      const pct = Math.min(100, (p.write_bytes_per_sec / maxRate) * 100);
      const rateMb = mbps(p.write_bytes_per_sec);
      const level = rateMb >= 2 ? "high" : rateMb >= 0.3 ? "mid" : "low";
      return `<div class="proc-row">
        <div class="proc-head">
          <span><b>${esc(p.name)}</b>${p.pids > 1 ? ` <span class="sub">（${p.pids} プロセス合算）</span>` : ""}</span>
          <span class="num"><span class="rate-level ${level}">${rateMb.toFixed(2)} MB/秒</span> <span class="sub">合計 ${fmtMb(p.total_written)}</span></span>
        </div>
        <span class="sbar"><span style="width:${pct.toFixed(1)}%"></span></span>
      </div>`;
    })
    .join("");
}

function rateRows() {
  if (!ssdState.rates.length) return `<div class="sub">読んでいます…</div>`;
  return ssdState.rates
    .map((r) => {
      const hist = ssdState.history.get(r.name) || [];
      const avg = hist.length
        ? hist.reduce((a, b) => a + b, 0) / hist.length
        : r.write_bytes_per_sec;
      const pct = Math.min(100, (mbps(r.write_bytes_per_sec) / 10) * 100);
      const level = mbps(avg) >= 3 ? "high" : mbps(avg) >= 1 ? "mid" : "low";
      const word = { high: "多い", mid: "ふつう〜やや多い", low: "少ない" }[
        level
      ];
      return `<div class="rate">
      <div class="rate-head"><b>ディスク ${esc(r.name)}</b><span class="rate-level ${level}">${word}</span></div>
      <div class="rate-num"><span class="big">${mbps(r.write_bytes_per_sec).toFixed(2)}</span> MB/秒 書き込み</div>
      <span class="sbar" role="meter" aria-label="今の書き込みの速さ（10 MB/秒で満杯）" aria-valuenow="${Math.round(pct)}" aria-valuemin="0" aria-valuemax="100"><span style="width:${pct}%"></span></span>
      <div class="sub">直近1分の平均 ${mbps(avg).toFixed(2)} MB/秒（このペースが1日続くと約 ${Math.round(perDayGb(avg))} GB）・読み込み ${mbps(r.read_bytes_per_sec).toFixed(2)} MB/秒</div>
    </div>`;
    })
    .join("");
}

const METHOD_WORD = {
  policy: "ポリシーで設定（どこから開いても効く）",
  userjs: "設定ファイル user.js に書く（プロファイルごと）",
  shortcut: "ショートカットに指定を足す（ほかのアプリのリンクから開いたときは効かない）",
};
const STATE_WORD = {
  small: "小さくしている",
  default: "ブラウザーにまかせている（最初の設定）",
  partial: "一部だけ小さくしている",
  none: "設定する場所が見つからない",
};

function browserRow(p) {
  const canSet = p.state !== "none";
  return `<div class="pol">
      <div class="pol-head"><b>${esc(p.name)}</b>
        <span class="pol-tag ${p.tested ? "ok" : "warn"}">${p.tested ? "この PC で効くのを確かめた" : "この PC では未確認"}</span></div>
      <div>${p.running ? `<span class="pol-tag ok">開いている</span> ` : ""}今：<span class="state">${esc(STATE_WORD[p.state] || p.state)}</span>${p.detail ? `<span class="sub">　${esc(p.detail)}</span>` : ""}</div>
      <div class="sub">${esc(METHOD_WORD[p.method] || "")}</div>
      <div class="row-btns">
        <button class="primary" data-pol="${p.id}" data-small="1" ${!canSet || p.state === "small" ? "disabled" : ""}>キャッシュを小さくする</button>
        <button data-pol="${p.id}" data-small="0" ${!canSet || p.state === "default" ? "disabled" : ""}>元に戻す</button>
        <button data-restart="${p.id}" ${p.running ? "" : "disabled"}>今すぐ開き直す</button>
      </div>
      <div class="sub">変えたあとは開き直すと効きます（「今すぐ開き直す」で自動。ただし開いていたタブは戻らないことがあります。タブを残すなら下のアドレスで自分で開き直してください）。手で開き直すなら：${p.restart.includes("://") ? `アドレス欄に <code>${esc(p.restart)}</code> と入れて Enter` : esc(p.restart)}</div>
    </div>`;
}

function policyRows() {
  if (!ssdState.policy.length) return `<div class="sub">読んでいます…</div>`;
  const inst = ssdState.policy.filter((p) => p.installed);
  const others = ssdState.policy.filter((p) => !p.installed);
  const settable = inst.filter((p) => p.state !== "none" && p.state !== "small");
  return `${inst.length ? `<div class="row-btns"><button class="primary" data-pol-all="1" ${settable.length ? "" : "disabled"}>入っているブラウザーを全部小さくする（${settable.length} 個）</button></div>` : ""}
    ${inst.map(browserRow).join("") || `<div class="sub">対応しているブラウザーが見つかりませんでした。</div>`}
    ${others.length ? `<details><summary>ほかに対応しているブラウザー（この PC には入っていない）${others.length} 個</summary><p class="sub">${others.map((p) => esc(p.name)).join("・")}</p></details>` : ""}`;
}

function renderSsd() {
  const el = document.getElementById("ssd");
  el.innerHTML = `
  <section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("ssd", 32)}</span>今の書き込みの速さ</h3>
    <p class="sub">2秒ごとに読みます。タスク マネージャーと同じ値です。何もしていないときに、ずっと「多い」なら、何かが書き続けています。</p>
    <div class="rates" id="ssdRates">${rateRows()}</div>
  </section>

  <section class="scard ssd-wide">
    <div style="display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap">
      <h3 style="margin:0"><span class="mic-wrap big">${iconSvg("pc", 32)}</span>今たくさん読み書きしているアプリ（多い順・通信も入る目安）</h3>
      <button class="small-btn" data-reset-proc="1">合計をリセット</button>
    </div>
    <p class="sub">Windows が数えている「アプリが書き出した量」です。<b>ディスクだけでなく、通信（通話・配信の受信など）やアプリ同士のやり取りも入る</b>ので、SSD に書いていないアプリが上に来ることがあります。あやしいアプリの見当をつけるのに使い、本当にディスクに書いているかは下の「自分で調べる」のリソース モニターで確かめてください。Chrome のようにプロセスがたくさんあるアプリは合算しています（管理者で開くとシステム系アプリも取れます）。</p>
    <div class="proc-list" id="ssdProcs">${procRateRows()}</div>
  </section>

  <section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("other", 32)}</span>ブラウザーのキャッシュを小さくする</h3>
    <p>ブラウザーで生配信（Twitch など）を見ると、数秒ごとの映像の切れ端（1つ 1〜2 MB）を全部 SSD に保存します。生配信は見返さないので、ほとんど無駄な書き込みです。</p>
    <p>キャッシュの大きさを 8 MB にすると、大きな映像の切れ端は保存されなくなり、配信はそのまま見られます。ふつうのページの表示が少し遅くなることがあります。</p>
    <div class="pols" id="ssdPol">${policyRows()}</div>
    <div class="msgline" id="ssdPolMsg"></div>
    <p class="sub">しくみ：Chrome・Edge・Brave・Vivaldi などは Windows のポリシー（自分のユーザーの <code>DiskCacheSize</code>）、Firefox の仲間は各プロファイルの <code>user.js</code>、ポリシーに対応していない Opera などはショートカットの <code>--disk-cache-size</code> で指示します。どれも管理者の権限は要りません。ポリシーで設定したブラウザーは、設定画面に「組織によって管理されています」と出ますが、この設定のためです。</p>
  </section>

  <section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("ssd", 32)}</span>SSD の寿命のしくみ</h3>
    <ul class="ssd-list">
      <li>SSD は、<b>書き込める量に上限</b>があります（メーカーが決めた「TBW」など）。読むだけでは減りません。</li>
      <li>「PC のスペック」の「ストレージの寿命」に出る<b>消耗度</b>は、この上限に対して、どれだけ使ったかの割合です（管理者で開くと見られます）。</li>
      <li>100% になっても、すぐ壊れるわけではありません。ただ、メーカーが保証する目安を超えるので、大事なファイルは別の場所にもコピーしておくと安心です。</li>
      <li>「あと何年」の目安は、今までの書き込みの速さから出しています。書き込みを減らせば、もっと長く持ちます。</li>
    </ul>
  </section>

  <section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("other", 32)}</span>書き込みが多くなる、よくある原因</h3>
    <dl class="skv">
      <dt>ブラウザーで生配信</dt><dd>映像の切れ端を全部保存します。見ている間、1時間に約 2.8 GB（下の例で測った値）。上の「キャッシュを小さくする」で止められます。</dd>
      <dt>記録する拡張機能</dt><dd>見たサイトや時間を記録し続ける拡張機能（例：StayFree）は、記録のデータを書き続けます。使っていなければ消すのが一番です。</dd>
      <dt>ゲームの入れ直し・更新</dt><dd>1回で数十 GB 書くことがあります。ゲームの置き場を、もう1つの SSD や HDD にすると分けられます。</dd>
      <dt>録画・動画の編集</dt><dd>1時間で数 GB〜数十 GB。保存先を別のドライブにすると分けられます。</dd>
      <dt>ドライバーの不具合</dt><dd>2026年11月に、AMD の Radeon のドライバー（26.10 系の <code>atieclxx.exe</code>）と NVIDIA のドライバー（621.14・621.24 の <code>NVDisplay.Container.exe</code>）で、何もしていなくても書き込み続ける不具合が報告されました。AMD は 26.11.1 で直っています。ドライバーは最新にしておくと安心です。</dd>
    </dl>
  </section>

  <section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("pc", 32)}</span>何が書いているか、自分で調べる</h3>
    <ol class="ssd-list">
      <li>上の「今の書き込みの速さ」で、多いかどうかを見る</li>
      <li>キーボードの <kbd>Windows</kbd> + <kbd>R</kbd> を押して <code>resmon</code> と入れ、Enter（リソース モニターが開きます）</li>
      <li>「ディスク」タブの「ディスク活動のプロセス」で、「書き込み (B/秒)」の見出しを押して多い順に並べる</li>
      <li>下の「ディスク活動」で、どのファイルに書いているかも分かります（ブラウザーの <code>Cache</code>、拡張機能の <code>IndexedDB</code> など）</li>
    </ol>
  </section>

  <section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("ssd", 32)}</span>例：この PC で調べて直したこと（2026年11月22日）</h3>
    <dl class="skv">
      <dt>気づいたこと</dt><dd>C: の SSD の消耗度が 52%。電源が入っていた 1年10か月で 132 TB 書いていて、1日あたり約 188 GB でした。</dd>
      <dt>記事の不具合？</dt><dd>この PC は AMD の GPU で、ドライバーはすでに直った版（11月17日）。<code>atieclxx.exe</code> の書き込みは 0 GB で、原因ではありませんでした。</dd>
      <dt>調べ方</dt><dd>Windows の記録（ETW）で、ディスクに本当に書かれた量をファイルごとに 2 分間数えました。</dd>
      <dt>分かった原因</dt><dd>Chrome が生配信の映像の切れ端（MPEG-TS）を保存していたのが約35%、拡張機能 StayFree の記録が約9%。どちらも怪しい物ではなく、ふだんの使い方による書き込みでした。</dd>
      <dt>したこと</dt><dd>StayFree を消す（記録データ 249 MB）。Chrome のキャッシュを 8 MB にする。</dd>
      <dt>結果</dt><dd>5分間に保存される映像の切れ端が 138 個（275 MB）→ <b>0 個</b>。C: への書き込みが 2.69 MB/秒 → <b>1.12 MB/秒（約6割減）</b>。配信はそのまま見られました。</dd>
    </dl>
  </section>`;
}

async function loadSsd() {
  try {
    ssdState.policy = await invoke("get_cache_policy");
  } catch {
    ssdState.policy = [];
  }
  renderSsd();
  pollRates();
}

async function pollRates() {
  try {
    const [rates, procs] = await Promise.all([
      invoke("get_disk_rates"),
      invoke("get_proc_io_rates").catch(() => []),
    ]);
    ssdState.rates = rates;
    ssdState.procRates = procs;
    for (const r of ssdState.rates) {
      const h = ssdState.history.get(r.name) || [];
      h.push(r.write_bytes_per_sec);
      while (h.length > 30) h.shift(); // 2秒 × 30 = 1分
      ssdState.history.set(r.name, h);
    }
    const box = document.getElementById("ssdRates");
    if (box) box.innerHTML = rateRows();
    const pbox = document.getElementById("ssdProcs");
    if (pbox) pbox.innerHTML = procRateRows();
  } catch (e) {
    const box = document.getElementById("ssdRates");
    if (box)
      box.innerHTML = `<div class="sub">読めませんでした: ${esc(String(e))}</div>`;
  }
}

const ssdVisible = () => !document.getElementById("tab-ssd").hidden;
setInterval(() => {
  if (ssdVisible() && !document.hidden) pollRates();
}, 2000);

/** ブラウザーを閉じて、タブごと開き直す。ask=true なら先に聞く */
async function restartBrowsers(ids, ask = false) {
  const names = ids.map((id) => ssdState.policy.find((p) => p.id === id)?.name || id);
  const q = `${names.join("・")} を今すぐ開き直しますか？\n\nブラウザーをいったん終わらせて開き直します。開いていたタブは戻らないことがあります（ブラウザーの「履歴」→「最近閉じたタブ」から開き直せます）。入力の途中の文字（フォームやメールの下書きなど）は消えます。タブを残したいときは、ブラウザーのアドレス欄に restart の文字（例：chrome://restart）を入れて、自分で開き直してください。`;
  if (!confirm(ask ? `設定を変えました。${q}` : q)) return;
  const msg = document.getElementById("ssdPolMsg");
  const done = [];
  for (const id of ids) {
    setMsg(msg, `${ssdState.policy.find((p) => p.id === id)?.name || id} を開き直しています…`);
    try {
      done.push(await invoke("restart_browser", { id }));
    } catch (err) {
      done.push(String(err));
    }
  }
  ssdState.policy = await invoke("get_cache_policy").catch(() => ssdState.policy);
  const box = document.getElementById("ssdPol");
  if (box) box.innerHTML = policyRows();
  setMsg(document.getElementById("ssdPolMsg"), done.join(" / "), "good");
}

document.getElementById("ssd").addEventListener("click", async (e) => {
  if (e.target.closest("[data-reset-proc]")) {
    await invoke("reset_proc_io_totals").catch(() => {});
    ssdState.procRates = [];
    const pbox = document.getElementById("ssdProcs");
    if (pbox) pbox.innerHTML = procRateRows();
    return;
  }
  const rs = e.target.closest("[data-restart]");
  if (rs) {
    await restartBrowsers([rs.dataset.restart]);
    return;
  }
  const all = e.target.closest("[data-pol-all]");
  const b = e.target.closest("[data-pol]");
  if (!all && !b) return;
  const msg = document.getElementById("ssdPolMsg");
  setMsg(msg, "書いています…");
  const ids = all
    ? ssdState.policy.filter((p) => p.installed && p.state !== "none" && p.state !== "small").map((p) => p.id)
    : [b.dataset.pol];
  const small = all ? true : b.dataset.small === "1";
  const errors = [];
  for (const id of ids) {
    try {
      ssdState.policy = await invoke("set_cache_policy", { id, small });
    } catch (err) {
      errors.push(String(err));
    }
  }
  if (errors.length) ssdState.policy = await invoke("get_cache_policy").catch(() => ssdState.policy);
  document.getElementById("ssdPol").innerHTML = policyRows();
  const m2 = document.getElementById("ssdPolMsg");
  if (errors.length) setMsg(m2, errors.join(" / "), "err");
  else setMsg(m2, small ? "小さくしました。それぞれのブラウザーを開き直すと効きます" : "元に戻しました。ブラウザーを開き直すと効きます", "good");
});
  const running = ssdState.policy.filter((p) => ids.includes(p.id) && p.running).map((p) => p.id);
  if (!errors.length && running.length) await restartBrowsers(running, true);
