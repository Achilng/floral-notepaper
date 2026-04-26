using System.IO;

namespace 花笺.Models;

public class AppConfig
{
    public string NotesDirectory { get; set; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments),
        "花笺", "notes");

    public string HotKey { get; set; } = "Ctrl+Space";
}
