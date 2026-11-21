# モニターの DDC/CI で「画面がついていた時間」(VCP 0xC0) が取れるか試す。読むだけで、モニターの設定は変えない。
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class Ddc {
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct PHYSICAL_MONITOR { public IntPtr h; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string desc; }
  public delegate bool MonEnum(IntPtr hMon, IntPtr hdc, IntPtr rect, IntPtr data);
  [DllImport("user32.dll")] public static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonEnum cb, IntPtr data);
  [DllImport("dxva2.dll")] public static extern bool GetNumberOfPhysicalMonitorsFromHMONITOR(IntPtr hMon, out uint n);
  [DllImport("dxva2.dll")] public static extern bool GetPhysicalMonitorsFromHMONITOR(IntPtr hMon, uint n, [Out] PHYSICAL_MONITOR[] arr);
  [DllImport("dxva2.dll")] public static extern bool GetVCPFeatureAndVCPFeatureReply(IntPtr h, byte code, IntPtr type, out uint cur, out uint max);
  [DllImport("dxva2.dll")] public static extern bool DestroyPhysicalMonitors(uint n, PHYSICAL_MONITOR[] arr);
  public static string Probe() {
    var sb = new System.Text.StringBuilder();
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (hMon, hdc, r, d) => {
      uint n; if (!GetNumberOfPhysicalMonitorsFromHMONITOR(hMon, out n) || n == 0) { sb.AppendLine("physical monitor なし"); return true; }
      var arr = new PHYSICAL_MONITOR[n];
      if (!GetPhysicalMonitorsFromHMONITOR(hMon, n, arr)) { sb.AppendLine("取得失敗"); return true; }
      foreach (var m in arr) {
        uint cur, max;
        bool ok = GetVCPFeatureAndVCPFeatureReply(m.h, 0xC0, IntPtr.Zero, out cur, out max);
        bool ok2; uint b, bm; ok2 = GetVCPFeatureAndVCPFeatureReply(m.h, 0x10, IntPtr.Zero, out b, out bm);
        sb.AppendLine(m.desc + " | 使用時間(0xC0): " + (ok ? cur + " (max " + max + ")" : "非対応 err=" + Marshal.GetLastWin32Error()) + " | 明るさ(0x10): " + (ok2 ? b + "/" + bm : "取れない"));
      }
      DestroyPhysicalMonitors(n, arr);
      return true;
    }, IntPtr.Zero);
    return sb.ToString();
  }
}
"@
[Ddc]::Probe()
