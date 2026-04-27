using System.Windows;
using System.Windows.Interop;
using 花笺.Helpers;

namespace 花笺.Services;

public class DesktopEmbedService
{
    private readonly Dictionary<IntPtr, IntPtr> _originalParents = [];

    public bool TryEmbed(Window window)
    {
        var hwnd = new WindowInteropHelper(window).Handle;
        if (hwnd == IntPtr.Zero)
            return false;

        var workerW = FindDesktopWorkerW();
        if (workerW == IntPtr.Zero)
            return false;

        _originalParents.TryAdd(hwnd, Win32Interop.GetParent(hwnd));

        Win32Interop.SetParent(hwnd, workerW);
        if (Win32Interop.GetParent(hwnd) != workerW)
            return false;

        window.Topmost = false;
        return true;
    }

    public void Restore(Window window)
    {
        var hwnd = new WindowInteropHelper(window).Handle;
        if (hwnd == IntPtr.Zero)
            return;

        var parent = _originalParents.GetValueOrDefault(hwnd, IntPtr.Zero);
        Win32Interop.SetParent(hwnd, parent);
        _originalParents.Remove(hwnd);
    }

    private static IntPtr FindDesktopWorkerW()
    {
        var progman = Win32Interop.FindWindow(Win32Interop.PROGMAN_CLASS, null);
        if (progman != IntPtr.Zero)
            Win32Interop.SendMessage(progman, Win32Interop.WM_SPAWN_WORKERW, IntPtr.Zero, IntPtr.Zero);

        var workerW = IntPtr.Zero;
        Win32Interop.EnumWindows((topHandle, _) =>
        {
            var shellView = Win32Interop.FindWindowEx(
                topHandle,
                IntPtr.Zero,
                Win32Interop.SHELLDLL_DEFVIEW_CLASS,
                null);

            if (shellView == IntPtr.Zero)
                return true;

            workerW = Win32Interop.FindWindowEx(
                IntPtr.Zero,
                topHandle,
                Win32Interop.WORKERW_CLASS,
                null);

            return workerW == IntPtr.Zero;
        }, IntPtr.Zero);

        return workerW != IntPtr.Zero ? workerW : progman;
    }
}
