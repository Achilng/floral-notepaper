using System.Windows;
using 花笺.Models;
using 花笺.Services;
using 花笺.ViewModels;
using 花笺.Views;

namespace 花笺;

public partial class App : Application
{
    private HotkeyService? _hotkeyService;
    private NotePadManager? _notePadManager;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        var config = new AppConfig();
        var metadataService = new MetadataService(config.NotesDirectory);
        var noteService = new NoteService(metadataService);

        _notePadManager = new NotePadManager(noteService);
        var viewModel = new MainViewModel(noteService, _notePadManager);

        var mainWindow = new MainWindow(viewModel);
        mainWindow.Show();

        _hotkeyService = new HotkeyService();
        _hotkeyService.QuickNoteRequested += _notePadManager.CreateQuickNote;
        _hotkeyService.Register(mainWindow);
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _notePadManager?.CloseAll();
        _hotkeyService?.Dispose();
        base.OnExit(e);
    }
}
