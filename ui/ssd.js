// 「SSD の書き込み」タブ。今の書き込みの速さ、ブラウザーのキャッシュの設定、SSD を長持ちさせる説明。
// index.html の state・invoke・esc・setMsg と、icons.js の iconSvg を使う。

const ssdState = { rates: [], history: new Map(), policy: [], timer: null };

const mbps = (b) => b / 1024 / 1024;
const perDayGb = (bps) => (bps * 86400) / 1024 ** 3;

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

function policyRows() {
  const list = ssdState.policy.filter((p) => p.installed);
  if (!list.length)
    return `<div class="sub">Chrome も Edge も見つかりませんでした。</div>`;
  return list
    .map((p) => {
      const small = p.size != null && p.size <= 16_000_000;
      const state =
        p.size == null
          ? "ブラウザーにまかせている（最初の設定）"
          : small
            ? `小さくしている（${Math.round(p.size / 1e6)} MB）`
            : `${Math.round(p.size / 1e6)} MB に決めてある`;
      const restart = p.id === "chrome" ? "chrome://restart" : "edge://restart";
      return `<div class="pol">
      <div><b>${esc(p.name)}</b>：<span class="state">${esc(state)}</span></div>
      <div class="row-btns">
        <button class="primary" data-pol="${p.id}" data-small="1" ${small ? "disabled" : ""}>キャッシュを小さくする</button>
        <button data-pol="${p.id}" data-small="0" ${p.size == null ? "disabled" : ""}>元に戻す</button>
      </div>
      <div class="sub">変えたあとは、アドレス欄に <code>${restart}</code> と入れて Enter で開き直すと効きます（窓を閉じるだけだと、裏で動き続けて効かないことがあります）。</div>
    </div>`;
    })
    .join("");
}

function renderSsd() {
  const el = document.getElementById("ssd");
  el.innerHTML = `
  <section class="scard ssd-wide">
    <h3><span class="mic-wrap big">${iconSvg("ssd", 32)}</span>今の書き込みの速さ</h3>
    <p class="sub">2秒ごとに読みます。タスク マネージャーと同じ値です。何もしていないときに、ずっと「多い」なら、何かが書き続けています。</p>
    <div class="rates" id="ssdRates">${rateRows()}</div>
  </section>

  <section class="scard">
    <h3><span class="mic-wrap big">${iconSvg("other", 32)}</span>ブラウザーのキャッシュを小さくする</h3>
    <p>ブラウザーで生配信（Twitch など）を見ると、数秒ごとの映像の切れ端（1つ 1〜2 MB）を全部 SSD に保存します。生配信は見返さないので、ほとんど無駄な書き込みです。</p>
    <p>キャッシュの大きさを 8 MB にすると、大きな映像の切れ端は保存されなくなり、配信はそのまま見られます。ふつうのページの表示が少し遅くなることがあります。</p>
    <div class="pols" id="ssdPol">${policyRows()}</div>
    <div class="msgline" id="ssdPolMsg"></div>
    <p class="sub">しくみ：Windows のポリシー（自分のユーザーの設定 <code>DiskCacheSize</code>）でブラウザーに指示します。管理者の権限は要りません。ブラウザーの設定画面に「組織によって管理されています」と出ますが、この設定のためです。</p>
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
    ssdState.rates = await invoke("get_disk_rates");
    for (const r of ssdState.rates) {
      const h = ssdState.history.get(r.name) || [];
      h.push(r.write_bytes_per_sec);
      while (h.length > 30) h.shift(); // 2秒 × 30 = 1分
      ssdState.history.set(r.name, h);
    }
    const box = document.getElementById("ssdRates");
    if (box) box.innerHTML = rateRows();
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

document.getElementById("ssd").addEventListener("click", async (e) => {
  const b = e.target.closest("[data-pol]");
  if (!b) return;
  const msg = document.getElementById("ssdPolMsg");
  try {
    ssdState.policy = await invoke("set_cache_policy", {
      id: b.dataset.pol,
      small: b.dataset.small === "1",
    });
    document.getElementById("ssdPol").innerHTML = policyRows();
    setMsg(
      document.getElementById("ssdPolMsg"),
      b.dataset.small === "1"
        ? "小さくしました。ブラウザーを開き直すと効きます"
        : "元に戻しました。ブラウザーを開き直すと効きます",
      "good",
    );
  } catch (err) {
    setMsg(msg, String(err), "err");
  }
});
