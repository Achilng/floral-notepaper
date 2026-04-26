using System.IO;
using System.Text.Json;
using 花笺.Models;

namespace 花笺.Services;

public class MetadataService
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    private readonly string _notesDir;
    private readonly string _metadataPath;
    private NoteMetadataStore _store = new();

    public MetadataService(string notesDirectory)
    {
        _notesDir = notesDirectory;
        _metadataPath = Path.Combine(_notesDir, "metadata.json");
    }

    public string NotesDirectory => _notesDir;

    public void EnsureDirectoryExists()
    {
        Directory.CreateDirectory(_notesDir);
    }

    public List<Note> Load()
    {
        EnsureDirectoryExists();

        if (!File.Exists(_metadataPath))
        {
            _store = new NoteMetadataStore();
            Save();
            return _store.Notes;
        }

        var json = File.ReadAllText(_metadataPath);
        _store = JsonSerializer.Deserialize<NoteMetadataStore>(json, JsonOptions) ?? new NoteMetadataStore();
        return _store.Notes;
    }

    public void Save()
    {
        EnsureDirectoryExists();
        var json = JsonSerializer.Serialize(_store, JsonOptions);
        File.WriteAllText(_metadataPath, json);
    }

    public void AddNote(Note note)
    {
        _store.Notes.Insert(0, note);
        Save();
    }

    public void RemoveNote(string noteId)
    {
        _store.Notes.RemoveAll(n => n.Id == noteId);
        Save();
    }

    public void UpdateNote(Note note)
    {
        var index = _store.Notes.FindIndex(n => n.Id == note.Id);
        if (index >= 0)
        {
            _store.Notes[index] = note;
            Save();
        }
    }
}
