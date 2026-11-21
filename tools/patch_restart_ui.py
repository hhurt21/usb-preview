"""ssd.js: ブラウザーを開き直すボタンと、小さくしたあとの「今すぐ開き直す？」 (一度だけ流す)。"""
import re
import sys

p = r"D:\claude\usb-watch\ui\ssd.js"
t = open(p, encoding="utf-8").read()
nl = "\r\n" if "\r\n" in t else "\n"
t = t.replace("\r\n", "\n")


def rep(old, new):
    global t
    if t.count(old) != 1:
        sys.exit(f"match {t.count(old)}: {old[:60]!r}")
    t = t.replace(old, new)


# 行に「開き直す」ボタンと、開いているかの印
m = re.search(r"<button data-pol=\"\$\{p\.id\}\" data-small=\"0\"[^\n]*</button>", t)
if not m:
    sys.exit("revert button not found")
t = t[:m.end()] + '\n        <button data-restart="${p.id}" ${p.running ? "" : "disabled"}>今すぐ開き直す</button>' + t[m.end():]
rep('<div>今：<span class="state">', '<div>${p.running ? `<span class="pol-tag ok">開いている</span> ` : ""}今：<span class="state">')
rep("<div class=\"sub\">変えたあと：", "<div class=\"sub\">変えたあとは開き直すと効きます（「今すぐ開き直す」で自動。開いていたタブは戻ります）。手で開き直すなら：")

# クリック: 開き直す
rep("""  const all = e.target.closest("[data-pol-all]");""", """  const rs = e.target.closest("[data-restart]");
  if (rs) {
    await restartBrowsers([rs.dataset.restart]);
    return;
  }
  const all = e.target.closest("[data-pol-all]");""")

# 小さくしたあと、開いているブラウザーがあれば開き直すか聞く
m = re.search(r"  if \(errors\.length\) setMsg\(m2, errors\.join\(\" / \"\), \"err\"\);\n  else setMsg\([^\n]*\n(?:[^\n]*\n)?", t)
if not m:
    sys.exit("result message not found")
t = t[:m.end()] + """  const running = ssdState.policy.filter((p) => ids.includes(p.id) && p.running).map((p) => p.id);
  if (!errors.length && running.length) await restartBrowsers(running, true);
""" + t[m.end():]

rep("""document.getElementById("ssd").addEventListener("click", async (e) => {""", """/** ブラウザーを閉じて、タブごと開き直す。ask=true なら先に聞く */
async function restartBrowsers(ids, ask = false) {
  const names = ids.map((id) => ssdState.policy.find((p) => p.id === id)?.name || id);
  const q = `${names.join("・")} を今すぐ開き直しますか？\\n\\n開いていたタブは開き直したあとに戻ります。ただし、入力の途中の文字（フォームやメールの下書きなど）は消えることがあります。`;
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

document.getElementById("ssd").addEventListener("click", async (e) => {""")
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
