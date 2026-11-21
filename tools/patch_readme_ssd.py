"""README に「SSD の書き込み」タブと「ストレージの寿命」の説明を足す (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\README.md"
t = open(p, encoding="utf-8").read()
lines = t.split("\n")
i = next((k for k, l in enumerate(lines) if l.startswith("| PC のスペック ")), None)
j = next((k for k, l in enumerate(lines) if l.startswith("### 抜いて当てる")), None)
if i is None or j is None:
    sys.exit("anchor not found")
lines.insert(i + 1, "| SSD の書き込み | 今のディスクへの書き込みの速さ（2秒ごと）、Chrome・Edge のキャッシュを小さくするボタン、SSD を長持ちさせる説明 | 要らない |")
section = """### SSD の書き込み

SSD は書き込める量に上限があるので、何がどれだけ書いているかを見て、無駄な書き込みを減らすためのページです。「PC のスペック」の「ストレージの寿命」の箱からも移れます。

![SSD の書き込み](docs/images/ssd.png)

- **今の書き込みの速さ**：ディスクごとに 2秒ごとに出します（タスク マネージャーと同じ値）。直近1分の平均と、「このペースが1日続くと何 GB か」も出します
- **ブラウザーのキャッシュを小さくする**：ブラウザーで生配信を見ると、映像の切れ端（1つ 1〜2 MB）を全部 SSD に保存します。ボタンを押すと、Chrome・Edge のディスク キャッシュを 8 MB にして、大きな切れ端を保存させないようにします（自分のユーザーのポリシー `DiskCacheSize`。管理者の権限は要りません。「元に戻す」で消せます）。変えたあとは `chrome://restart` で開き直すと効きます
- **説明**：SSD の寿命のしくみ、書き込みが多くなるよくある原因（生配信・記録する拡張機能・ゲームの更新・録画・ドライバーの不具合）、リソース モニターで自分で調べる方法、この PC で調べて直した例（2026年11月22日：C: への書き込みが 2.69 → 1.12 MB/秒）

PC のスペックには **ストレージの寿命** の箱もあります。ディスクごとに、健康状態（良好・注意・危険）・寿命の残りのバー・今までと同じ使い方ならあと何年か・1日あたりの書き込み量を出します（管理者で開いたとき。普通の権限なら箱の中に「管理者で開き直す」ボタンが出ます）。

"""
lines[j + 1:j + 1] = []
lines[j:j] = section.split("\n")
t = "\n".join(lines)
t = t.replace("| `src-tauri/src/smart.rs`",
              "| `src-tauri/src/diskrate.rs` | 今のディスクへの書き込みの速さ（パフォーマンス カウンター） |\n"
              "| `src-tauri/src/browser.rs` | Chrome・Edge のディスク キャッシュの大きさ（ポリシー） |\n"
              "| `src-tauri/src/smart.rs`", 1)
t = t.replace("| `ui/spec.js`", "| `ui/ssd.js` | SSD の書き込みタブ |\n| `ui/spec.js`", 1)
open(p, "w", encoding="utf-8", newline="").write(t)
print("ok")
