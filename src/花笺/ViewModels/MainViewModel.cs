using System.Collections.ObjectModel;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using 花笺.Models;
using 花笺.Services;

namespace 花笺.ViewModels;

public partial class MainViewModel : ObservableObject
{
    private readonly NoteService _noteService;
    private readonly NotePadManager _notePadManager;
    private readonly DispatcherTimer _saveTimer;

    [ObservableProperty]
    private ObservableCollection<Note> _notes = [];

    [ObservableProperty]
    private Note? _selectedNote;

    [ObservableProperty]
    private string _editorContent = string.Empty;

    [ObservableProperty]
    private string _noteTitle = string.Empty;

    [ObservableProperty]
    private bool _isSaved = true;

    [ObservableProperty]
    private int _wordCount;

    [ObservableProperty]
    private string _statusTime = string.Empty;

    [ObservableProperty]
    private bool _hasSelectedNote;

    [ObservableProperty]
    private bool _showDeleteConfirm;

    [ObservableProperty]
    private bool _isEditMode = true;

    [ObservableProperty]
    private string _previewHtml = string.Empty;

    public MainViewModel(NoteService noteService, NotePadManager notePadManager)
    {
        _noteService = noteService;
        _notePadManager = notePadManager;

        _saveTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(800) };
        _saveTimer.Tick += (_, _) =>
        {
            _saveTimer.Stop();
            PerformSave();
        };

        _notePadManager.NoteSavedToLibrary += OnNoteSavedExternally;

        LoadNotes();
    }

    private void LoadNotes()
    {
        var allNotes = _noteService.GetAllNotes();
        Notes = new ObservableCollection<Note>(allNotes);
    }

    partial void OnSelectedNoteChanged(Note? value)
    {
        ShowDeleteConfirm = false;
        IsEditMode = true;

        if (value == null)
        {
            EditorContent = string.Empty;
            NoteTitle = string.Empty;
            HasSelectedNote = false;
            StatusTime = string.Empty;
            WordCount = 0;
            PreviewHtml = string.Empty;
            return;
        }

        HasSelectedNote = true;
        NoteTitle = value.Title;
        EditorContent = _noteService.LoadContent(value);
        UpdateStatusTime(value);
        UpdateWordCount();
    }

    partial void OnEditorContentChanged(string value)
    {
        if (SelectedNote == null) return;
        IsSaved = false;
        UpdateWordCount();
        _saveTimer.Stop();
        _saveTimer.Start();
    }

    partial void OnNoteTitleChanged(string value)
    {
        if (SelectedNote == null) return;
        IsSaved = false;
        _saveTimer.Stop();
        _saveTimer.Start();
    }

    partial void OnIsEditModeChanged(bool value)
    {
        if (!value)
            PreviewHtml = MarkdownService.ToHtml(EditorContent);
    }

    private void PerformSave()
    {
        if (SelectedNote == null) return;

        if (SelectedNote.Title != NoteTitle)
        {
            _noteService.UpdateTitle(SelectedNote, NoteTitle);
            RefreshNoteInList(SelectedNote);
        }

        _noteService.SaveContent(SelectedNote, EditorContent);
        UpdateStatusTime(SelectedNote);
        IsSaved = true;
    }

    [RelayCommand]
    private void NewNote()
    {
        var note = _noteService.CreateNote();
        Notes.Insert(0, note);
        SelectedNote = note;
    }

    [RelayCommand]
    private void RequestDelete()
    {
        if (SelectedNote == null) return;
        ShowDeleteConfirm = true;
    }

    [RelayCommand]
    private void ConfirmDelete()
    {
        if (SelectedNote == null) return;

        var note = SelectedNote;
        var index = Notes.IndexOf(note);

        _noteService.DeleteNote(note);
        Notes.Remove(note);

        ShowDeleteConfirm = false;

        if (Notes.Count > 0)
            SelectedNote = Notes[Math.Min(index, Notes.Count - 1)];
        else
            SelectedNote = null;
    }

    [RelayCommand]
    private void CancelDelete()
    {
        ShowDeleteConfirm = false;
    }

    [RelayCommand]
    private void SwitchToEdit()
    {
        IsEditMode = true;
    }

    [RelayCommand]
    private void SwitchToPreview()
    {
        IsEditMode = false;
    }

    [RelayCommand]
    private void InsertMarkdown(string tag)
    {
        if (!IsEditMode) return;
        OnInsertMarkdownRequested?.Invoke(tag);
    }

    public event Action<string>? OnInsertMarkdownRequested;

    private void UpdateWordCount()
    {
        if (string.IsNullOrEmpty(EditorContent))
        {
            WordCount = 0;
            return;
        }

        int chinese = 0, english = 0;
        bool inWord = false;
        foreach (var c in EditorContent)
        {
            if (c >= '一' && c <= '龥')
            {
                chinese++;
                inWord = false;
            }
            else if (char.IsLetter(c))
            {
                if (!inWord) { english++; inWord = true; }
            }
            else
            {
                inWord = false;
            }
        }
        WordCount = chinese + english;
    }

    private void UpdateStatusTime(Note note)
    {
        StatusTime = note.UpdatedAt.ToString("yyyy-MM-dd HH:mm");
    }

    private void RefreshNoteInList(Note note)
    {
        var index = Notes.IndexOf(note);
        if (index >= 0)
        {
            Notes.RemoveAt(index);
            Notes.Insert(index, note);
            SelectedNote = note;
        }
    }

    [RelayCommand]
    private void QuickNote()
    {
        _notePadManager.CreateQuickNote();
    }

    [RelayCommand]
    private void PinNote()
    {
        if (SelectedNote != null)
            _notePadManager.PinNote(SelectedNote);
    }

    private void OnNoteSavedExternally(Note note)
    {
        var allNotes = _noteService.GetAllNotes();
        Notes = new ObservableCollection<Note>(allNotes);
        SelectedNote = Notes.FirstOrDefault(n => n.Id == note.Id);
    }

    public string GetDisplayTitle(Note note)
    {
        if (!string.IsNullOrWhiteSpace(note.Title))
            return note.Title;

        var content = _noteService.LoadContent(note);
        if (!string.IsNullOrWhiteSpace(content))
        {
            var firstLine = content.Split('\n', 2)[0].TrimStart('#', ' ');
            if (!string.IsNullOrWhiteSpace(firstLine))
                return firstLine.Length > 30 ? firstLine[..30] + "..." : firstLine;
        }

        return "无标题笔记";
    }
}
