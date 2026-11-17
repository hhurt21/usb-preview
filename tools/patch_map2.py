"""map.js の直し: モニターの絵の比率・画面の台数・USB の中の部品を二重に出さない。"""
import re
import sys

p = r"D:\claude\usb-watch\ui\map.js"
t = open(p, encoding="utf-8").read()
nl = "\r\n" if "\r\n" in t else "\n"
t = t.replace("\r\n", "\n")


def sub1(pattern, repl):
    global t
    new, n = re.subn(pattern, repl, t, count=1, flags=re.S)
    if n != 1:
        sys.exit(f"no match: {pattern[:70]!r}")
    t = new


# 1) USB: 親がふつうの USB 機器 (ハブでない) なら、その中の部品なので出さない
sub1(r'(if \(\s*!u\.startsWith\("USB\\\\"\) \|\|\s*u\.includes\("&MI_"\) \|\|)',
     r'\1\n      u.includes("&IG_") ||\n      isUsbChild(d) ||')
sub1(r'(function mapName\(d\) \{)',
     'function isUsbChild(d) {\n  const p = mapDev(d.parent);\n  if (!p) return false;\n  const u = up(p.id);\n'
     '  return u.startsWith("USB\\\\") && !u.includes("ROOT_HUB") && !p.is_hub;\n}\n\n\\1')

# 2) 画面の台数はモニターの数で数える
sub1(r'(function cardHtml\(key, c, showVirtual, extraTop = ""\) \{)',
     'function cardHtml(key, c, showVirtual, extraTop = "", countText = null) {')
sub1(r'\$\{c\.items\.length\} 台', '${countText ?? `${c.items.length} 台`}')
sub1(r'(cardHtml\("screen", cats\.screen, showVirtual, monitorsHtml\(\))\)',
     r'\1, `${mapState.ov.monitors.length} 台`)')

# 3) モニターの絵: 画面の部分だけを位置と大きさで置き、台はその下に付ける。全体は幅に合わせて縦横比を保つ
sub1(r'const left = \(\(m\.x - minX\) / W\) \* 100,\s*top = \(\(m\.y - minY\) / H\) \* 100;',
     'const HH = H * 1.22;\n    const left = ((m.x - minX) / W) * 100,\n      top = ((m.y - minY) / HH) * 100;')
sub1(r'const w = \(m\.width / W\) \* 100,\s*h = \(m\.height / H\) \* 100;',
     'const w = (m.width / W) * 100,\n      h = (m.height / HH) * 100;')
sub1(r'style="aspect-ratio:\$\{W\} / \$\{H \* 1\.12\}"',
     'style="aspect-ratio:${W} / ${H * 1.22};width:min(100%, ${Math.round((300 * W) / (H * 1.22))}px)"')

open(p, "w", encoding="utf-8", newline="").write(t.replace("\n", nl))
print("ok")
