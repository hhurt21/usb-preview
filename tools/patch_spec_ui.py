"""index.html に PC のスペックタブ・USB の木・音のメーターの見た目をつなぐ (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\ui\index.html"
t = open(p, encoding="utf-8").read()
nl = "\r\n" if "\r\n" in t else "\n"
t = t.replace("\r\n", "\n")


def rep(old, new):
    global t
    n = t.count(old)
    if n != 1:
        sys.exit(f"match {n}: {old[:70]!r}")
    t = t.replace(old, new)


# 全体図の並び: USB の木は右の列を縦に長く使う
rep('''        grid-template-areas:
          "screen screen usb"
          "audio pc input"
          "bt net store"
          "other internal internal";''', '''        grid-template-areas:
          "screen screen usb"
          "audio pc usb"
          "audio input usb"
          "bt net store"
          "other internal internal";''')

CSS = '''      .mtools {'''
NEW_CSS = '''      /* USB の木 */
      .ugroup {
        margin-top: 8px;
      }
      .ugroup-title {
        display: flex;
        gap: 8px;
        align-items: center;
        font-size: 14px;
        font-weight: 600;
        color: var(--muted);
        padding: 4px 0;
      }
      .ugroup-title .ic {
        color: var(--accent);
        flex: none;
      }
      .utree,
      .utree ul {
        list-style: none;
        margin: 0;
        padding: 0;
      }
      .utree ul {
        margin-left: 14px;
        padding-left: 12px;
        border-left: 2px solid var(--card-line);
      }
      .utree li {
        margin: 2px 0;
      }
      .utree-row.is-hub .mname {
        font-weight: 600;
        color: var(--muted);
      }
      .uport {
        display: inline-block;
        font-size: 12px;
        font-weight: 600;
        color: var(--accent);
        border: 1px solid var(--accent);
        border-radius: 6px;
        padding: 0 6px;
        margin-right: 6px;
      }
      /* 音 */
      .asub {
        margin: 10px 0 4px;
        font-size: 14px;
        color: var(--muted);
      }
      .abadges {
        display: flex;
        gap: 6px;
        flex-wrap: wrap;
        margin-top: 2px;
      }
      .abadge {
        font-size: 12px;
        font-weight: 700;
        padding: 0 8px;
        border-radius: 999px;
      }
      .abadge.main {
        color: var(--accent-text);
        background: var(--accent);
      }
      .abadge.comm {
        color: var(--accent);
        border: 1px solid var(--accent);
      }
      .ameter {
        display: block;
        height: 6px;
        margin-top: 4px;
        border-radius: 3px;
        background: var(--line);
        overflow: hidden;
      }
      .ameter > span {
        display: block;
        height: 100%;
        background: var(--ok);
        transition: width 0.15s linear;
      }
      .aapps {
        font-size: 13px;
        color: var(--text);
      }
      .pc-more {
        margin-top: 10px;
        width: 100%;
      }
      /* PC のスペック */
      .spec {
        display: grid;
        gap: 14px;
        grid-template-columns: repeat(auto-fill, minmax(min(100%, 420px), 1fr));
        align-items: start;
      }
      .scard {
        background: var(--panel);
        border: 1px solid var(--line);
        border-radius: 14px;
        padding: 12px 16px;
      }
      .scard h3 {
        display: flex;
        gap: 10px;
        align-items: center;
        margin: 0 0 8px;
        font-size: 17px;
      }
      .skv {
        display: grid;
        grid-template-columns: max-content 1fr;
        gap: 6px 14px;
        margin: 0;
      }
      .skv dt {
        color: var(--muted);
      }
      .skv dd {
        margin: 0;
        overflow-wrap: anywhere;
      }
      .sbar {
        display: block;
        height: 6px;
        margin-top: 4px;
        border-radius: 3px;
        background: var(--line);
        overflow: hidden;
      }
      .sbar > span {
        display: block;
        height: 100%;
        background: var(--accent);
      }
      .sbar.low > span {
        background: var(--bad);
      }
      .mtools {'''
rep(CSS, NEW_CSS)

rep('''        <button role="tab" data-tab="map" aria-selected="false">全体図</button>''',
    '''        <button role="tab" data-tab="map" aria-selected="false">全体図</button>
        <button role="tab" data-tab="spec" aria-selected="false">PC のスペック</button>''')
rep('''      <section id="tab-tree" hidden>''', '''      <section id="tab-spec" hidden>
        <div class="mtools">
          <button id="sReload">読み直す</button>
          <button class="primary" id="sCopy">文字でコピー</button>
          <span class="sub" id="sInfo"></span>
        </div>
        <div class="msgline" id="sCopyMsg"></div>
        <div class="spec" id="spec"><div class="empty">読んでいます…</div></div>
      </section>

      <section id="tab-tree" hidden>''')
rep('''          for (const t of ["list", "map", "tree", "hist", "settings"])''',
    '''          for (const t of ["list", "map", "spec", "tree", "hist", "settings"])''')
rep('''          if (b.dataset.tab === "map") loadMap(!mapState.ov);''',
    '''          if (b.dataset.tab === "map") loadMap(!mapState.ov);
          if (b.dataset.tab === "spec") {
            if (specState.data) renderSpec();
            else loadSpec(false);
          }''')
rep('''    <script src="map.js"></script>''', '''    <script src="map.js"></script>
    <script src="spec.js"></script>''')
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
