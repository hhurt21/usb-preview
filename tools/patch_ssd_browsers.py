"""ssd.js のブラウザーの欄を、たくさんのブラウザー (ポリシー・user.js・ショートカット) に合わせて置き換える。"""
import re
import sys

p = r"D:\claude\usb-watch\ui\ssd.js"
t = open(p, encoding="utf-8").read()
nl = "\r\n" if "\r\n" in t else "\n"
t = t.replace("\r\n", "\n")

NEW_ROWS = r'''const METHOD_WORD = {
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
      <div>今：<span class="state">${esc(STATE_WORD[p.state] || p.state)}</span>${p.detail ? `<span class="sub">　${esc(p.detail)}</span>` : ""}</div>
      <div class="sub">${esc(METHOD_WORD[p.method] || "")}</div>
      <div class="row-btns">
        <button class="primary" data-pol="${p.id}" data-small="1" ${!canSet || p.state === "small" ? "disabled" : ""}>キャッシュを小さくする</button>
        <button data-pol="${p.id}" data-small="0" ${!canSet || p.state === "default" ? "disabled" : ""}>元に戻す</button>
      </div>
      <div class="sub">変えたあと：${p.restart.includes("://") ? `アドレス欄に <code>${esc(p.restart)}</code> と入れて Enter` : esc(p.restart)}</div>
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
'''

m = re.search(r"function policyRows\(\) \{.*?\n\}\n", t, re.S)
if not m:
    sys.exit("policyRows not found")
t = t[:m.start()] + NEW_ROWS + t[m.end():]

m = re.search(r'document\.getElementById\("ssd"\)\.addEventListener\("click", async \(e\) => \{.*?\n\}\);\n', t, re.S)
if not m:
    sys.exit("click handler not found")
NEW_CLICK = r'''document.getElementById("ssd").addEventListener("click", async (e) => {
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
'''
t = t[:m.start()] + NEW_CLICK + t[m.end():]

# 説明の文も、いろいろなブラウザー向けに
t = t.replace("Chrome・Edge のディスク キャッシュを", "ブラウザーのディスク キャッシュを")
t = t.replace(
    "しくみ：Windows のポリシー（自分のユーザーの設定 <code>DiskCacheSize</code>）でブラウザーに指示します。管理者の権限は要りません。ブラウザーの設定画面に「組織によって管理されています」と出ますが、この設定のためです。",
    "しくみ：Chrome・Edge・Brave・Vivaldi などは Windows のポリシー（自分のユーザーの <code>DiskCacheSize</code>）、Firefox の仲間は各プロファイルの <code>user.js</code>、ポリシーに対応していない Opera などはショートカットの <code>--disk-cache-size</code> で指示します。どれも管理者の権限は要りません。ポリシーで設定したブラウザーは、設定画面に「組織によって管理されています」と出ますが、この設定のためです。")
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
