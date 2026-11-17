"""index.html に全体図タブをつなぐ置換 (一度だけ流す)。改行は元のファイルに合わせる。"""
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


CSS = """      .empty {
        color: var(--muted);
        padding: 16px;
      }
      /* ---------- 全体図 ---------- */
      :root {
        --ic-fill: #cfe8de;
        --card-screen: #2f6fb3;
        --card-line: #9fb3aa;
        --bezel: #2a2f2c;
        --screen-a: #dbeafe;
        --screen-b: #bfdbfe;
        --screen-text: #10243f;
      }
      @media (prefers-color-scheme: dark) {
        :root {
          --ic-fill: #24493c;
          --card-line: #4d5f57;
          --bezel: #0b0d0c;
          --screen-a: #1e3a5f;
          --screen-b: #172f4d;
          --screen-text: #e3efff;
        }
      }
      .map {
        --narrow: 0;
        position: relative;
        display: grid;
        gap: 22px;
        grid-template-columns: minmax(0, 1fr) minmax(250px, 0.8fr) minmax(0, 1fr);
        grid-template-areas:
          "screen screen usb"
          "audio pc input"
          "bt net store"
          "other internal internal";
        align-items: start;
      }
      @media (max-width: 980px) {
        .map {
          --narrow: 1;
          grid-template-columns: minmax(0, 1fr);
          grid-template-areas: "pc" "screen" "usb" "audio" "input" "bt" "net" "store" "other" "internal";
        }
      }
      .mlines {
        position: absolute;
        inset: 0;
        pointer-events: none;
        z-index: 0;
        overflow: visible;
      }
      .mlines path {
        stroke: var(--card-line);
        stroke-width: 2.5;
        stroke-dasharray: 6 6;
        fill: none;
      }
      .mlines circle {
        fill: var(--card-line);
      }
      .mcard {
        position: relative;
        z-index: 1;
        background: var(--panel);
        border: 1px solid var(--line);
        border-radius: 14px;
        padding: 12px 14px;
        min-width: 0;
      }
      .mcard h3 {
        display: flex;
        align-items: center;
        gap: 10px;
        margin: 0 0 8px;
        font-size: 17px;
      }
      .mcount {
        margin-left: auto;
        font-size: 14px;
        color: var(--muted);
        font-weight: 600;
      }
      .mic-wrap {
        color: var(--accent);
        display: inline-flex;
        flex: none;
      }
      .ic .f {
        fill: var(--ic-fill);
      }
      .mlist {
        display: grid;
        gap: 4px;
      }
      .mrow {
        display: flex;
        align-items: center;
        gap: 10px;
        width: 100%;
        text-align: left;
        white-space: normal;
        padding: 6px 8px;
        min-height: 44px;
        border-radius: 8px;
        border: 1px solid transparent;
        background: transparent;
      }
      .mrow:hover {
        background: var(--bg);
        border-color: var(--line);
      }
      .mrow.flap {
        border-color: var(--bad);
        border-width: 2px;
      }
      .mtext {
        display: grid;
        flex: 1;
        min-width: 0;
      }
      .mname {
        font-weight: 600;
        overflow-wrap: anywhere;
      }
      .mvnote {
        margin-top: 6px;
      }
      .mcard details {
        margin-top: 4px;
      }
      .mcard summary {
        cursor: pointer;
        padding: 4px 0;
      }
      .mpc {
        text-align: center;
        border: 2px solid var(--accent);
        align-self: center;
      }
      .pc-pic {
        color: var(--accent);
      }
      .pc-title {
        justify-content: center;
        font-size: 20px;
      }
      .pc-spec {
        display: grid;
        gap: 6px;
        text-align: left;
      }
      .pc-line {
        display: flex;
        gap: 8px;
        align-items: center;
        color: var(--accent);
      }
      .pc-line > span {
        color: var(--text);
        overflow-wrap: anywhere;
      }
      .mons-wrap {
        padding: 6px 4px 4px;
      }
      .mons {
        position: relative;
        width: 100%;
        max-height: 300px;
      }
      .mon {
        position: absolute;
        padding: 0 6px 18%;
        border: none;
        background: transparent;
        white-space: normal;
        min-height: 0;
      }
      .mon::after {
        content: "";
        position: absolute;
        left: 42%;
        right: 42%;
        bottom: 4%;
        height: 12%;
        background: var(--bezel);
        border-radius: 0 0 6px 6px;
      }
      .mon-screen {
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        gap: 2px;
        height: 100%;
        border: 7px solid var(--bezel);
        border-radius: 8px;
        background: linear-gradient(135deg, var(--screen-a), var(--screen-b));
        color: var(--screen-text);
        padding: 6px;
        text-align: center;
      }
      .mon:hover .mon-screen {
        outline: 3px solid var(--accent);
      }
      .mon-name {
        font-weight: 700;
        font-size: 16px;
      }
      .mon-spec,
      .mon-gpu {
        font-size: 13px;
      }
      .mtools {
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
        align-items: center;
        margin-bottom: 12px;
      }"""

rep("""      .empty {
        color: var(--muted);
        padding: 16px;
      }""", CSS)

# タブ: 全体図を「一覧」の次に
rep("""        <button role="tab" data-tab="tree" aria-selected="false">""",
    """        <button role="tab" data-tab="map" aria-selected="false">全体図</button>
        <button role="tab" data-tab="tree" aria-selected="false">""")
rep("""      <section id="tab-tree" hidden>""",
    """      <section id="tab-map" hidden>
        <div class="mtools">
          <label class="filters" style="margin:0"><input type="checkbox" id="mVirtual" /> 仮想の機器も出す</label>
          <input type="text" id="mText" placeholder="名前で探す" aria-label="名前で探す" />
          <button id="mReload">読み直す</button>
          <span class="sub" id="mInfo"></span>
        </div>
        <div class="map" id="map"><div class="empty">読んでいます…</div></div>
      </section>

      <section id="tab-tree" hidden>""")
rep("""          for (const t of ["list", "tree", "hist", "settings"])""",
    """          for (const t of ["list", "map", "tree", "hist", "settings"])""")
rep("""          if (b.dataset.tab === "hist") loadHistory();""",
    """          if (b.dataset.tab === "hist") loadHistory();
          if (b.dataset.tab === "map") loadMap(!mapState.ov);""")

# くわしく・操作を USB 以外の機器でも開けるように
rep("""          state.controllers.find((d) => up(d.id) === u)
        );""", """          state.controllers.find((d) => up(d.id) === u) ||
          (state.all || []).find((d) => up(d.id) === u)
        );""")
rep("""      <dt>外れた回数</dt><dd>10分で ${d.removed_10min || 0} 回 ／ アプリを開いてから ${d.removed_total || 0} 回</dd>""",
    """      ${up(d.id).startsWith("USB\\\\") ? `<dt>外れた回数</dt><dd>10分で ${d.removed_10min || 0} 回 ／ アプリを開いてから ${d.removed_total || 0} 回</dd>` : ""}""")
rep("""        const isInput =""", """        const isCore = /^(Display|System|Processor|Computer|SCSIAdapter|HDC|DiskDrive|Volume|Firmware|Net)$/.test(
          d.class || "",
        );
        const isInput =""")
rep("""      ${d.is_hub && d.status""", """      ${isCore && d.status !== "disabled" ? `<p class="flapnote">この機器（${esc(d.class)}）を無効にすると、画面が消える・ディスクが使えない・ネットにつながらないなど、PC が正しく動かなくなることがあります。</p>` : ""}
      ${d.is_hub && d.status""")
rep("""            isInput &&
            !confirm(
              "キーボードやマウスかもしれません。無効にすると操作できなくなることがあります。無効にしますか？",""",
    """            (isInput || isCore) &&
            !confirm(
              "無効にすると、操作できなくなったり PC が正しく動かなくなったりすることがあります。本当に無効にしますか？",""")
rep("""    </script>
  </body>""", """    </script>
    <script src="icons.js"></script>
    <script src="map.js"></script>
  </body>""")
open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
