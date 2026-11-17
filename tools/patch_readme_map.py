"""README に全体図の説明を足す (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\README.md"
t = open(p, encoding="utf-8").read()
lines = t.split("\n")

i = next((k for k, l in enumerate(lines) if l.startswith("| つながり方 ")), None)
j = next((k for k, l in enumerate(lines) if l.startswith("### 抜いて当てる")), None)
if i is None or j is None:
    sys.exit("anchor not found")

row = ("| 全体図 | USB だけでなく、画面・音・Bluetooth・ネットワーク・ディスク・入力機器など、"
       "PC につながっている全部の機器を、この PC を真ん中にした図で見せます | 要らない |")
section = """### 全体図

USB だけでなく、PC につながっている全部の機器を1枚の図にします。真ん中が「この PC」（CPU・スレッド数・メモリ・GPU）で、周りに種類ごとの箱を並べて点線でつなぎます。機器の絵は種類ごとに描いた SVG です。

![全体図（画面と USB）](docs/images/map.png)

| 箱 | 出す物 |
| --- | --- |
| 画面 | モニターを Windows の画面設定と同じ並び方で描きます。名前・解像度・リフレッシュレート・つなぎ方（HDMI / DisplayPort など）・メインかどうか・つながっている GPU を出します |
| USB | ポートにさした機器。中身（キーボード・ゲームパッド・音の機器・Bluetooth など）から絵を選びます |
| 音 | 鳴らす側（スピーカー・ヘッドホン）と録る側（マイク） |
| 入力 | キーボード・マウス・ゲームパッド。「HID キーボード デバイス」のような部品は、もとの機器（例：AK820MAX）にまとめます |
| Bluetooth | アダプターと、ペアリングした機器 |
| ネットワーク | つながっているかと速さ |
| 保存 | ディスクの容量とつなぎ方（NVMe / USB など） |
| その他 | シリアル（COM）・カメラ・プリンターなど |
| Windows の中の部品 | マザーボードやドライバーの部品など。種類ごとにたたんであります |

![全体図（この PC と周りの箱）](docs/images/map-pc.png)

- **仮想の機器**（Voicemeeter・VMware・Parsec など、アプリや Windows が作った機器）は、最初は数だけ出します。「仮想の機器も出す」にすると並びます。機器の親をたどって、本物の部品（PCI・ACPI）に着く前に Windows が作った機器（ROOT・SWD）に着いたら、仮想とみなしています
- 「名前で探す」に文字を入れると、全部の箱をまとめて絞り込みます
- 機器を押すと「くわしく・操作」が開きます。GPU・ディスク・ネットワークなど、無効にすると PC が正しく動かなくなる物には、強い注意が出ます
- 機器がつながったり外れたりすると、自動で読み直します。解像度を変えたときなどは「読み直す」を押してください

![全体図（下のほう）](docs/images/map-bottom.png)
"""
lines.insert(i + 1, row)
j += 1
lines[j:j] = section.split("\n")
open(p, "w", encoding="utf-8", newline="").write("\n".join(lines))
print("ok")
