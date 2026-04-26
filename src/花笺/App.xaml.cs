using System.Windows;
using 花笺.Models;
using 花笺.Services;
using 花笺.ViewModels;
using 花笺.Views;

namespace 花笺;

public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        var config = new AppConfig();
        var metadataService = new MetadataService(config.NotesDirectory);
        var noteService = new NoteService(metadataService);
        var viewModel = new MainViewModel(noteService);

        var mainWindow = new MainWindow(viewModel);
        mainWindow.Show();
    }
}
