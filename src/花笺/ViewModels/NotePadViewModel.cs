using System.Collections.ObjectModel;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using 花笺.Models;
using 花笺.Services;

namespace 花笺.ViewModels;

public partial class NotePadViewModel : ObservableObject
{
    private readonly NoteService _noteService;
    private readonly DispatcherTimer _saveTimer;

    [ObservableProperty] private string _title = string.Empty;
    [ObservableProperty] private string _content = string.Empty;
    [ObservableProperty] private bool _isTile;
    [ObservableProperty] private bool _isTopmost;
    [ObservableProperty] private bool _isNoteListMode;
    [ObservableProperty] private ObservableCollection<Note> _noteList = [];

    public Note? LinkedNote { get; private set; }

    public event Action? CloseRequested;
    public event Action<Note>? SavedToNotes;

    public NotePadViewModel(NoteService noteService, Note? existingNote = null)
    {
        _noteService = noteService;

        _saveTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(800) };
        _saveTimer.Tick += (_, _) =>
        {
            _saveTimer.Stop();
            AutoSave();
        };

        if (existingNote != null)
        {
            LinkedNote = existingNote;
            Title = existingNote.Title;
            Content = _noteService.LoadContent(existingNote);
            IsTile = true;
            IsTopmost = true;
        }
    }

    partial void OnContentChanged(string value)
    {
        if (LinkedNote != null)
        {
            _saveTimer.Stop();
            _saveTimer.Start();
        }
    }

    partial void OnTitleChanged(string value)
    {
        if (LinkedNote != null)
        {
            _saveTimer.Stop();
            _saveTimer.Start();
        }
    }

    private void AutoSave()
    {
        if (LinkedNote == null) return;

        if (LinkedNote.Title != Title)
            _noteService.UpdateTitle(LinkedNote, Title);

        _noteService.SaveContent(LinkedNote, Content);
    }

    [RelayCommand]
    private void TogglePin()
    {
        if (!IsTile)
        {
            EnsureLinked();
            IsTile = true;
            IsTopmost = true;
        }
        else
        {
            IsTile = false;
            IsTopmost = false;
        }
    }

    [RelayCommand]
    private void SaveToNotesAndClose()
    {
        EnsureLinked();
        AutoSave();
        SavedToNotes?.Invoke(LinkedNote!);
        CloseRequested?.Invoke();
    }

    [RelayCommand]
    private void RequestClose()
    {
        if (LinkedNote != null)
        {
            AutoSave();
        }
        else if (!string.IsNullOrWhiteSpace(Content))
        {
            EnsureLinked();
            AutoSave();
        }

        CloseRequested?.Invoke();
    }

    [RelayCommand]
    private void ShowNoteList()
    {
        IsNoteListMode = true;
        NoteList = new ObservableCollection<Note>(_noteService.GetAllNotes());
    }

    [RelayCommand]
    private void ShowNewNote()
    {
        IsNoteListMode = false;
    }

    [RelayCommand]
    private void OpenExistingNote(Note note)
    {
        LinkedNote = note;
        Title = note.Title;
        Content = _noteService.LoadContent(note);
        IsNoteListMode = false;
    }

    [RelayCommand]
    private void ToggleTopmost()
    {
        IsTopmost = !IsTopmost;
    }

    private void EnsureLinked()
    {
        if (LinkedNote != null) return;

        var note = _noteService.CreateNote();
        if (!string.IsNullOrWhiteSpace(Title))
            _noteService.UpdateTitle(note, Title);
        _noteService.SaveContent(note, Content);
        LinkedNote = note;
    }
}
