using System.ComponentModel;
using System.Reflection;
using System.Windows;
using System.Windows.Threading;
using System.Xml;
using ICSharpCode.AvalonEdit;
using ICSharpCode.AvalonEdit.Highlighting;
using ICSharpCode.AvalonEdit.Highlighting.Xshd;
using 花笺.ViewModels;

namespace 花笺.Views;

public partial class MainWindow : Window
{
    private readonly MainViewModel _vm;
    private int _previewNavigationVersion;

    public MainWindow(MainViewModel viewModel)
    {
        InitializeComponent();
        _vm = viewModel;
        DataContext = _vm;

        LoadMarkdownHighlighting();

        _vm.PropertyChanged += OnViewModelPropertyChanged;
        _vm.OnInsertMarkdownRequested += HandleInsertMarkdown;
    }

    private void LoadMarkdownHighlighting()
    {
        var assembly = Assembly.GetExecutingAssembly();
        var resourceName = assembly.GetManifestResourceNames()
            .FirstOrDefault(n => n.EndsWith("MarkdownHighlighting.xshd"));

        if (resourceName == null) return;

        using var stream = assembly.GetManifestResourceStream(resourceName);
        if (stream == null) return;

        using var reader = new XmlTextReader(stream);
        var highlighting = HighlightingLoader.Load(reader, HighlightingManager.Instance);
        Editor.SyntaxHighlighting = highlighting;
    }

    private void OnViewModelPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (!_vm.IsEditMode &&
            (e.PropertyName == nameof(MainViewModel.IsEditMode) ||
             e.PropertyName == nameof(MainViewModel.PreviewHtml)))
        {
            QueuePreviewNavigation();
        }
    }

    private void QueuePreviewNavigation()
    {
        var version = ++_previewNavigationVersion;
        _ = NavigateToPreviewAsync(version);
    }

    private async Task NavigateToPreviewAsync(int version)
    {
        var html = _vm.PreviewHtml;
        if (string.IsNullOrEmpty(html)) return;

        try
        {
            await Dispatcher.InvokeAsync(() => { }, DispatcherPriority.Loaded);

            if (version != _previewNavigationVersion || _vm.IsEditMode)
                return;

            var ensureTask = await Dispatcher.InvokeAsync(
                () => PreviewBrowser.EnsureCoreWebView2Async());
            await ensureTask;

            if (version != _previewNavigationVersion || _vm.IsEditMode)
                return;

            await Dispatcher.InvokeAsync(() => PreviewBrowser.NavigateToString(html));
        }
        catch
        {
            // If WebView2 runtime is unavailable, keep the editor usable.
        }
    }

    private void HandleInsertMarkdown(string tag)
    {
        var editor = Editor;
        var doc = editor.Document;
        var caret = editor.CaretOffset;
        var selection = editor.TextArea.Selection;
        var selectedText = selection.IsEmpty ? string.Empty : selection.GetText();

        string before, after;
        bool wrapSelection = !string.IsNullOrEmpty(selectedText);

        switch (tag)
        {
            case "heading":
                var line = doc.GetLineByOffset(caret);
                var lineText = doc.GetText(line.Offset, line.Length);
                if (lineText.StartsWith("### "))
                    doc.Replace(line.Offset, 4, "");
                else if (lineText.StartsWith("## "))
                    doc.Replace(line.Offset, 3, "### ");
                else if (lineText.StartsWith("# "))
                    doc.Replace(line.Offset, 2, "## ");
                else
                    doc.Insert(line.Offset, "# ");
                return;

            case "bold":
                before = "**"; after = "**"; break;
            case "italic":
                before = "*"; after = "*"; break;
            case "code":
                if (selectedText.Contains('\n'))
                    { before = "```\n"; after = "\n```"; }
                else
                    { before = "`"; after = "`"; }
                break;
            case "list":
                doc.Insert(doc.GetLineByOffset(caret).Offset, "- ");
                return;
            case "quote":
                doc.Insert(doc.GetLineByOffset(caret).Offset, "> ");
                return;
            case "hr":
                var hrLine = doc.GetLineByOffset(caret);
                doc.Insert(hrLine.EndOffset, hrLine.Length == 0 ? "---\n" : "\n---\n");
                return;
            default:
                return;
        }

        if (wrapSelection)
        {
            var start = doc.GetOffset(selection.StartPosition.Location);
            var end = doc.GetOffset(selection.EndPosition.Location);
            if (start > end) (start, end) = (end, start);
            doc.Replace(start, end - start, before + selectedText + after);
            editor.CaretOffset = start + before.Length + selectedText.Length + after.Length;
        }
        else
        {
            doc.Insert(caret, before + after);
            editor.CaretOffset = caret + before.Length;
        }

        editor.Focus();
    }
}
