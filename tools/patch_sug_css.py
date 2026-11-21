"""index.html にアップグレードの提案の見た目を足す (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\ui\index.html"
t = open(p, encoding="utf-8").read()
nl = "\r\n" if "\r\n" in t else "\n"
t = t.replace("\r\n", "\n")
anchor = "      .sbar.low > span {"
if t.count(anchor) != 1:
    sys.exit("anchor not found")
css = """      .sug-card {
        grid-column: 1 / -1;
      }
      .sug-list {
        display: grid;
        gap: 10px;
        grid-template-columns: repeat(auto-fill, minmax(min(100%, 380px), 1fr));
      }
      .sug {
        border: 1px solid var(--line);
        border-left-width: 6px;
        border-radius: 10px;
        padding: 10px 12px;
        background: var(--bg);
      }
      .sug.now {
        border-left-color: var(--bad);
      }
      .sug.maybe {
        border-left-color: var(--off);
      }
      .sug.ok {
        border-left-color: var(--ok);
      }
      .sug-head {
        display: flex;
        gap: 8px;
        align-items: baseline;
        flex-wrap: wrap;
        margin-bottom: 4px;
      }
      .sug-level {
        font-size: 13px;
        font-weight: 700;
        padding: 0 8px;
        border-radius: 999px;
        white-space: nowrap;
      }
      .sug.now .sug-level {
        color: var(--bad);
        background: var(--bad-bg);
      }
      .sug.maybe .sug-level {
        color: var(--off);
        background: var(--off-bg);
      }
      .sug.ok .sug-level {
        color: var(--ok);
        background: var(--ok-bg);
      }
      .sug-why {
        color: var(--muted);
        font-size: 14px;
      }
      .sug-what {
        margin-top: 4px;
      }
"""
t = t.replace(anchor, css + anchor)
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
