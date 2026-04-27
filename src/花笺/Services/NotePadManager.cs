using System.Windows;
using 花笺.Models;
using 花笺.ViewModels;
using 花笺.Views;

namespace 花笺.Services;

public class NotePadManager
{
    private readonly NoteService _noteService;
    private readonly List<NotePadWindow> _windows = [];
    private int _staggerOffset;

    public event Action<Note>? NoteSavedToLibrary;

    public NotePadManager(NoteService noteService)
    {
        _noteService = noteService;
    }

    public void CreateQuickNote()
    {
        var vm = new NotePadViewModel(_noteService);
        var window = BuildWindow(vm);

        var screenW = SystemParameters.PrimaryScreenWidth;
        var screenH = SystemParameters.PrimaryScreenHeight;
        window.Left = (screenW - window.Width) / 2 + _staggerOffset;
        window.Top = (screenH - window.Height) / 2 + _staggerOffset;
        _staggerOffset = (_staggerOffset + 24) % 120;

        window.Show();
        window.Activate();
    }

    public void PinNote(Note note)
    {
        var existing = _windows.FirstOrDefault(w =>
            w.ViewModel.LinkedNote?.Id == note.Id);

        if (existing != null)
        {
            existing.Activate();
            return;
        }

        var vm = new NotePadViewModel(_noteService, note);
        var window = BuildWindow(vm);

        var main = Application.Current.MainWindow;
        if (main != null)
        {
            window.Left = main.Left + main.Width + 20;
            window.Top = main.Top + 50 + _staggerOffset;
        }
        _staggerOffset = (_staggerOffset + 24) % 120;

        window.Show();
    }

    private NotePadWindow BuildWindow(NotePadViewModel vm)
    {
        var window = new NotePadWindow(vm);

        vm.CloseRequested += () =>
        {
            _windows.Remove(window);
            window.Close();
        };

        vm.SavedToNotes += note => NoteSavedToLibrary?.Invoke(note);

        _windows.Add(window);
        window.Closed += (_, _) => _windows.Remove(window);

        return window;
    }

    public void CloseAll()
    {
        foreach (var window in _windows.ToList())
            window.Close();
        _windows.Clear();
    }
}
