"""README に USB の木・今使っている音の機器・PC のスペックタブの説明を足す (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\README.md"
t = open(p, encoding="utf-8").read()


def rep(old, new):
    global t
    n = t.count(old)
    if n != 1:
        sys.exit(f"match {n}: {old[:60]!r}")
    t = t.replace(old, new)


lines = t.split("\n")
i = next(k for k, l in enumerate(lines) if l.startswith("| 全体図 "))
lines.insert(i + 1, "| PC のスペック | OS・CPU・メモリ（枚数・速さ）・マザーボード・BIOS・GPU（ビデオメモリ・ドライバー）・ディスク（空き容量）・ネットワーク・画面・今使っている音の機器を1画面にまとめ、文字でコピーできます | 要らない |")
t = "\n".join(lines)

rep("| USB | ポートにさした機器。中身（キーボード・ゲームパッド・音の機器・Bluetooth など）から絵を選びます |",
    "| USB | コントローラー → ハブ → 機器 の木の形で、どのハブの何番の口に何がつながっているかを出します。機器の絵は中身（キーボード・ゲームパッド・音の機器・Bluetooth など）から選びます。機器がつながっていないハブは省きます |")
rep("| 音 | 鳴らす側（スピーカー・ヘッドホン）と録る側（マイク） |",
    "| 音 | 鳴らす側（スピーカー・ヘッドホン）と録る側（マイク）。Windows の既定の機器に「いつも使う」「通話で使う」の印をつけ、今の音の大きさをバーで、鳴らしている（録っている）アプリの名前も出します |")
rep("![全体図（この PC と周りの箱）](docs/images/map-pc.png)",
    "![全体図（この PC と周りの箱）](docs/images/map-pc.png)\n\n"
    "音の箱と USB の木です。下の画像では、いつも使うスピーカーで Overwatch・Voicemod が鳴っていて、通話用のヘッドセットは Discord が使っています。"
    "USB の木では、ハブの先のハブの、何番の口に何がつながっているかが分かります。\n\n"
    "![音の機器と USB の木](docs/images/map-usb-audio.png)")
rep("- 「名前で探す」に文字を入れると、全部の箱をまとめて絞り込みます",
    "- 既定の機器と、アプリが使っている音の機器は、仮想の機器（Voicemod など）でも必ず出します\n"
    "- 音の大きさのバーは、全体図を見ている間だけ 0.5 秒ごとに読みます\n"
    "- 「この PC」の箱の「くわしいスペックを見る」で、PC のスペックタブに移ります\n"
    "- 「名前で探す」に文字を入れると、全部の箱をまとめて絞り込みます")

spec_section = """### PC のスペック

OS・CPU・メモリ・マザーボード・BIOS・GPU・ディスク・ネットワーク・画面・今使っている音の機器を、1画面にまとめます。

![PC のスペック](docs/images/spec.png)

- **メモリ**：差してある枚数と差し込み口の数、1枚ずつの容量・種類（DDR4 / DDR5 など）・速さ
- **GPU**：ビデオメモリはレジストリの正しい値を使います（Windows の管理情報だと 4GB で頭打ちになるため）。アプリが作った仮想の画面アダプターは数だけ出します
- **ディスク**：ドライブごとの空きをバーで出し、空きが1割を切ると赤にします
- **文字でコピー**：全部を文字にしてコピーします。Discord などに貼って、PC の相談に使えます
- PC の名前・ユーザー名・シリアル番号・MAC アドレス・IP アドレスなど、人や機械を特定できる情報は集めません（画面にもコピーにも出ません）

"""
rep("### 抜いて当てる", spec_section + "### 抜いて当てる")

rep("| `src-tauri/src/store.rs`",
    "| `src-tauri/src/audio.rs` | 今使っている音の機器（既定の機器・音の大きさ・鳴らしているアプリ。Windows の音の仕組み Core Audio に直接聞く） |\n"
    "| `src-tauri/src/spec.rs` | PC のスペック（Windows の管理情報 CIM とレジストリ） |\n"
    "| `src-tauri/src/store.rs`")
rep("| `ui/icons.js`", "| `ui/spec.js` | PC のスペックタブ |\n| `ui/icons.js`")
open(p, "w", encoding="utf-8", newline="").write(t)
print("ok")
