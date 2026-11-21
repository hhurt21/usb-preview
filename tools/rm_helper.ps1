# 再起動マネージャー (Restart Manager) を PowerShell から呼ぶための型
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class Rm {
  [StructLayout(LayoutKind.Sequential)] public struct FILETIME { public uint Low; public uint High; }
  [StructLayout(LayoutKind.Sequential)] public struct RM_UNIQUE_PROCESS { public int dwProcessId; public FILETIME ProcessStartTime; }
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)] public static extern int RmStartSession(out uint h, int flags, string key);
  [DllImport("rstrtmgr.dll")] public static extern int RmEndSession(uint h);
  [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)] public static extern int RmRegisterResources(uint h, uint nFiles, string[] files, uint nApps, RM_UNIQUE_PROCESS[] apps, uint nSvc, string[] svcs);
  [DllImport("rstrtmgr.dll")] public static extern int RmShutdown(uint h, uint flags, IntPtr cb);
  [DllImport("rstrtmgr.dll")] public static extern int RmRestart(uint h, int reserved, IntPtr cb);
  [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint a, bool i, int pid);
  [DllImport("kernel32.dll")] public static extern bool GetProcessTimes(IntPtr h, out FILETIME c, out FILETIME e, out FILETIME k, out FILETIME u);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  public static string Run(int[] pids) {
    uint s; var key = Guid.NewGuid().ToString("N");
    int r = RmStartSession(out s, 0, key); if (r != 0) return "start " + r;
    var apps = new RM_UNIQUE_PROCESS[pids.Length];
    for (int i = 0; i < pids.Length; i++) {
      var h = OpenProcess(0x1000, false, pids[i]); FILETIME c, e, k, u; GetProcessTimes(h, out c, out e, out k, out u); CloseHandle(h);
      apps[i].dwProcessId = pids[i]; apps[i].ProcessStartTime = c;
    }
    r = RmRegisterResources(s, 0, null, (uint)apps.Length, apps, 0, null); if (r != 0) { RmEndSession(s); return "register " + r; }
    r = RmShutdown(s, 0, IntPtr.Zero); var sd = r;
    System.Threading.Thread.Sleep(1500);
    r = RmRestart(s, 0, IntPtr.Zero);
    RmEndSession(s);
    return "shutdown=" + sd + " restart=" + r;
  }
}
"@
