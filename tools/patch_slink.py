"""index.html: 提案を部品の名前から開く形にする見た目と案内 (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\ui\index.html"
t = open(p, encoding="utf-8").read()
nl = "\r\n" if "\r\n" in t else "\n"
t = t.replace("\r\n", "\n")


def rep(old, new):
    global t
    if t.count(old) != 1:
        sys.exit(f"match {t.count(old)}: {old[:60]!r}")
    t = t.replace(old, new)


rep("""      .sug-card {
        grid-column: 1 / -1;
      }
""", """      .slink {
        display: inline;
        padding: 0;
        min-height: 0;
        border: none;
        background: none;
        color: var(--accent);
        font-weight: 700;
        text-align: left;
        white-space: normal;
        text-decoration: underline;
        text-decoration-style: dotted;
        text-underline-offset: 4px;
        cursor: pointer;
      }
      .slink[aria-expanded="true"] {
        text-decoration-style: solid;
      }
      .sug-panel {
        display: grid;
        gap: 8px;
        margin-top: 12px;
        padding-top: 10px;
        border-top: 1px solid var(--line);
      }
      .sug-panel h4 {
        margin: 0;
        font-size: 15px;
      }
""")
rep("""          <span class="sub" id="sInfo"></span>
        </div>""", """          <span class="sub" id="sInfo"></span>
        </div>
        <p class="sub">点線の下線がある名前を押すと、その部品のアップグレードの提案が出ます。</p>""")
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
