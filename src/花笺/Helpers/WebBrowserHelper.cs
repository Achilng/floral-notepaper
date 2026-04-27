using System.Diagnostics;
using System.IO;
using Microsoft.Win32;

namespace 花笺.Helpers;

public static class WebBrowserHelper
{
    public static void SetIE11EmulationMode()
    {
        var exeName = Path.GetFileName(Process.GetCurrentProcess().MainModule?.FileName ?? "花笺.exe");
        const string keyPath = @"SOFTWARE\Microsoft\Internet Explorer\Main\FeatureControl\FEATURE_BROWSER_EMULATION";

        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(keyPath, true)
                            ?? Registry.CurrentUser.CreateSubKey(keyPath);
            key?.SetValue(exeName, 11001, RegistryValueKind.DWord);
        }
        catch
        {
            // Non-critical, preview will just render in legacy mode
        }
    }
}
