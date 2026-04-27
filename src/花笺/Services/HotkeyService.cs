using System.Windows;
using System.Windows.Interop;
using 花笺.Helpers;

namespace 花笺.Services;

public class HotkeyService : IDisposable
{
    private HwndSource? _source;
    private IntPtr _windowHandle;
    private const int HOTKEY_ID = 9001;
    private bool _registered;

    public event Action? QuickNoteRequested;

    public void Register(Window window)
    {
        var helper = new WindowInteropHelper(window);
        helper.EnsureHandle();
        _windowHandle = helper.Handle;
        _source = HwndSource.FromHwnd(_windowHandle);
        _source?.AddHook(WndProc);

        _registered = Win32Interop.RegisterHotKey(
            _windowHandle, HOTKEY_ID,
            Win32Interop.MOD_CONTROL, Win32Interop.VK_SPACE);
    }

    public bool IsRegistered => _registered;

    private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg == Win32Interop.WM_HOTKEY && wParam.ToInt32() == HOTKEY_ID)
        {
            QuickNoteRequested?.Invoke();
            handled = true;
        }
        return IntPtr.Zero;
    }

    public void Dispose()
    {
        if (_windowHandle != IntPtr.Zero && _registered)
            Win32Interop.UnregisterHotKey(_windowHandle, HOTKEY_ID);
        _source?.RemoveHook(WndProc);
    }
}
