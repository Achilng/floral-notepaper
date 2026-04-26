using System.IO;
using 花笺.Models;

namespace 花笺.Services;

public class NoteService
{
    private readonly MetadataService _metadata;

    public NoteService(MetadataService metadata)
    {
        _metadata = metadata;
    }

    public List<Note> GetAllNotes() => _metadata.Load();

    public Note CreateNote()
    {
        var note = new Note
        {
            Title = string.Empty,
            CreatedAt = DateTime.Now,
            UpdatedAt = DateTime.Now
        };
        note.FileName = BuildFileName(note);

        var filePath = GetFilePath(note);
        File.WriteAllText(filePath, string.Empty);

        _metadata.AddNote(note);
        return note;
    }

    public string LoadContent(Note note)
    {
        var path = GetFilePath(note);
        return File.Exists(path) ? File.ReadAllText(path) : string.Empty;
    }

    public void SaveContent(Note note, string content)
    {
        var path = GetFilePath(note);
        File.WriteAllText(path, content);
        note.UpdatedAt = DateTime.Now;
        _metadata.UpdateNote(note);
    }

    public void UpdateTitle(Note note, string newTitle)
    {
        var oldPath = GetFilePath(note);

        note.Title = newTitle;
        var newFileName = BuildFileName(note);

        if (note.FileName != newFileName)
        {
            var newPath = Path.Combine(_metadata.NotesDirectory, newFileName);
            if (File.Exists(oldPath))
                File.Move(oldPath, newPath);
            note.FileName = newFileName;
        }

        note.UpdatedAt = DateTime.Now;
        _metadata.UpdateNote(note);
    }

    public void DeleteNote(Note note)
    {
        var path = GetFilePath(note);
        if (File.Exists(path))
            File.Delete(path);
        _metadata.RemoveNote(note.Id);
    }

    private string GetFilePath(Note note) =>
        Path.Combine(_metadata.NotesDirectory, note.FileName);

    private static string BuildFileName(Note note)
    {
        if (string.IsNullOrWhiteSpace(note.Title))
            return $"{note.Id}.md";

        var safe = SanitizeFileName(note.Title);
        return $"{note.Id}_{safe}.md";
    }

    private static string SanitizeFileName(string name)
    {
        var invalid = Path.GetInvalidFileNameChars();
        var clean = new string(name.Where(c => !invalid.Contains(c)).ToArray());
        return clean.Length > 60 ? clean[..60] : clean;
    }
}
