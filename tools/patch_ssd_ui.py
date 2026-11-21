"""index.html に「SSD の書き込み」タブをつなぐ (一度だけ流す)。"""
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


rep('''        <button role="tab" data-tab="spec" aria-selected="false">PC のスペック</button>''',
    '''        <button role="tab" data-tab="spec" aria-selected="false">PC のスペック</button>
        <button role="tab" data-tab="ssd" aria-selected="false">SSD の書き込み</button>''')
rep('''      <section id="tab-tree" hidden>''', '''      <section id="tab-ssd" hidden>
        <div class="spec" id="ssd"><div class="empty">読んでいます…</div></div>
      </section>

      <section id="tab-tree" hidden>''')
rep('''          for (const t of ["list", "map", "spec", "tree", "hist", "settings"])''',
    '''          for (const t of ["list", "map", "spec", "ssd", "tree", "hist", "settings"])''')
rep('''          if (b.dataset.tab === "map") loadMap(!mapState.ov);''',
    '''          if (b.dataset.tab === "map") loadMap(!mapState.ov);
          if (b.dataset.tab === "ssd") loadSsd();''')
rep('''    <script src="spec.js"></script>''', '''    <script src="spec.js"></script>
    <script src="ssd.js"></script>''')
rep('''      .mtools {''', '''      /* SSD の書き込み */
      .ssd-wide {
        grid-column: 1 / -1;
      }
      .rates {
        display: grid;
        gap: 12px;
        grid-template-columns: repeat(auto-fill, minmax(min(100%, 300px), 1fr));
      }
      .rate {
        border: 1px solid var(--line);
        border-radius: 10px;
        padding: 10px 12px;
        background: var(--bg);
      }
      .rate-head {
        display: flex;
        justify-content: space-between;
        gap: 8px;
        align-items: baseline;
      }
      .rate-num .big {
        font-size: 26px;
        font-weight: 700;
        font-variant-numeric: tabular-nums;
      }
      .rate-level {
        font-size: 13px;
        font-weight: 700;
        padding: 0 8px;
        border-radius: 999px;
      }
      .rate-level.high {
        color: var(--bad);
        background: var(--bad-bg);
      }
      .rate-level.mid {
        color: var(--off);
        background: var(--off-bg);
      }
      .rate-level.low {
        color: var(--ok);
        background: var(--ok-bg);
      }
      .pols {
        display: grid;
        gap: 12px;
        margin: 8px 0;
      }
      .pol {
        display: grid;
        gap: 6px;
        border: 1px solid var(--line);
        border-radius: 10px;
        padding: 10px 12px;
      }
      .ssd-list {
        margin: 0;
        padding-left: 22px;
        display: grid;
        gap: 6px;
      }
      .scard code,
      .scard kbd {
        font-family: Consolas, "Cascadia Mono", monospace;
        font-size: 14px;
        background: var(--bg);
        border: 1px solid var(--line);
        border-radius: 4px;
        padding: 0 4px;
      }
      .scard p {
        margin: 4px 0 8px;
      }
      .mtools {''')
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
