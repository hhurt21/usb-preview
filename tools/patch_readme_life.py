"""README に「使った時間・年数」と「アップグレードの提案」の説明を足す (一度だけ流す)。"""
import sys

p = r"D:\claude\usb-watch\README.md"
t = open(p, encoding="utf-8").read()
anchor = "- **文字でコピー**："
if t.count(anchor) != 1:
    sys.exit("anchor not found")
add = """- **使った時間・年数**：
  - モニター：画面がついていた合計時間（モニターと PC が話す仕組み DDC/CI で読みます。対応していないモニターは「記録なし」）
  - SSD：電源が入っていた合計時間・電源を入れた回数・書き込んだ量・消耗度・温度（NVMe SSD 自身の健康記録を読みます。管理者で開いたときだけ）
  - CPU・GPU の発売からの年数（目安）、今の Windows を入れてから、BIOS が作られてから
  - 時間は「1年10か月（16,797 時間）」のように、年と月でも出します
- **アップグレードの提案**：調べた結果から「すぐやるとよい」「余裕があれば」「今のままで十分」に分けて、理由と一緒に出します。見ているのは、ディスクの空き・SSD の消耗と警告・メモリの量と速さ（XMP / EXPO が切れていないか）・CPU（AM4 なら 5700X3D / 5800X3D など）・GPU（画面の解像度とリフレッシュレートに足りるか）・BIOS とドライバーの古さ・モニターの使用時間・USB の不調・LAN の速さです。製品名はこのアプリを作った時点（2026年）の目安です
"""
t = t.replace(anchor, add + anchor)
t = t.replace("| `src-tauri/src/spec.rs`",
              "| `src-tauri/src/smart.rs` | NVMe SSD の健康記録（電源が入っていた時間・消耗度など）を直接読む。読むだけ |\n| `src-tauri/src/spec.rs`", 1)
open(p, "w", encoding="utf-8", newline="").write(t)
print("ok")
