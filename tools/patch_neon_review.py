"""neon が足した画面の、説明の直し (一度だけ流す)。
- アプリごとの読み書き: 通信も混ざるので「SSD に書いている」と言わない
- RAM の解放: 逆効果になりやすいことを書く。一時ファイルは 24 時間より古い物だけ消すことを書く
"""
import sys


def patch(path, pairs):
    t = open(path, encoding="utf-8").read()
    nl = "\r\n" if "\r\n" in t else "\n"
    t = t.replace("\r\n", "\n")
    for old, new in pairs:
        if t.count(old) != 1:
            sys.exit(f"{path}: match {t.count(old)}: {old[:50]!r}")
        t = t.replace(old, new)
    open(path, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))


patch(r"D:\claude\usb-watch\ui\ssd.js", [
    ("今 SSD に書いているアプリ（多い順）", "今たくさん読み書きしているアプリ（多い順・通信も入る目安）"),
    ("Chrome のようにプロセスがたくさんあるアプリは合算しています。リソース モニターを開かなくても、どのアプリが書いているかここで見られます（管理者で開くとシステム系アプリも取れます）。",
     "Windows が数えている「アプリが書き出した量」です。<b>ディスクだけでなく、通信（通話・配信の受信など）やアプリ同士のやり取りも入る</b>ので、SSD に書いていないアプリが上に来ることがあります。あやしいアプリの見当をつけるのに使い、本当にディスクに書いているかは下の「自分で調べる」のリソース モニターで確かめてください。Chrome のようにプロセスがたくさんあるアプリは合算しています（管理者で開くとシステム系アプリも取れます）。"),
])

patch(r"D:\claude\usb-watch\ui\diag.js", [
    ("""    <div class="row-btns" style="margin-top:10px">
      <button class="primary" id="btnBoostRam">今すぐ RAM を解放</button>
      <button id="btnBoostAll">RAM ＋ 一時ファイルをまとめて解放</button>
    </div>""",
     """    <p class="sub" style="margin-top:8px"><b>注意：</b>Windows は空いているメモリを、ファイルを速く読むための置き場（キャッシュ）に使っています。解放すると一時的に「空き」は増えますが、あとでアプリがメモリを取り戻すときに読み直しや、ページファイル（SSD）とのやり取りが増え、<b>ゲームがカクついたり SSD への書き込みが増えたりすることがあります</b>。メモリが本当に足りなくて困っているとき（使用率がずっと 90% 以上など）だけ使うのがおすすめです。</p>
    <div class="row-btns" style="margin-top:10px">
      <button class="primary" id="btnBoostRam">今すぐ RAM を解放</button>
      <button id="btnBoostAll">RAM ＋ 一時ファイルをまとめて解放</button>
    </div>
    <p class="sub">一時ファイルは、%TEMP% の中で <b>24 時間以上さわられていない物だけ</b>を消します（インストールの途中の物や、動いているアプリが使っている物を守るため）。</p>"""),
])
print("ok")
