using System.ComponentModel;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using 花笺.Models;
using 花笺.Services;
using 花笺.ViewModels;

namespace 花笺.Views;

public partial class NotePadWindow : Window
{
    private readonly NotePadViewModel _vm;
    private readonly DesktopEmbedService _desktopEmbedService;

    public NotePadViewModel ViewModel => _vm;

    public NotePadWindow(NotePadViewModel viewModel, DesktopEmbedService desktopEmbedService)
    {
        InitializeComponent();
        _vm = viewModel;
        _desktopEmbedService = desktopEmbedService;
        DataContext = _vm;

        Loaded += (_, _) =>
        {
            if (!_vm.IsTile)
                ContentEditor.Focus();
        };
        Closing += (_, _) => _desktopEmbedService.Restore(this);
        _vm.PropertyChanged += OnViewModelPropertyChanged;
    }

    private void OnViewModelPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(NotePadViewModel.IsDesktopEmbedded))
            ApplyDesktopEmbedding();
    }

    private void ApplyDesktopEmbedding()
    {
        if (!_vm.IsDesktopEmbedded)
        {
            _desktopEmbedService.Restore(this);
            return;
        }

        if (_desktopEmbedService.TryEmbed(this))
            return;

        _vm.IsDesktopEmbedded = false;
        _vm.IsTopmost = true;
    }

    private void DragHandle_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (e.OriginalSource is TextBox) return;
        DragMove();
    }

    private void MoveGrip_DragDelta(object sender, DragDeltaEventArgs e)
    {
        Left += e.HorizontalChange;
        Top += e.VerticalChange;
    }

    private void TitleInput_PreviewMouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (!TitleInput.IsFocused)
        {
            e.Handled = true;
            TitleInput.Focus();
            TitleInput.SelectAll();
        }
    }

    private void TitleInput_GotFocus(object sender, RoutedEventArgs e)
    {
        TitleInput.Cursor = Cursors.IBeam;
    }

    private void ResizeGrip_DragDelta(object sender, DragDeltaEventArgs e)
    {
        var newWidth = Width + e.HorizontalChange;
        var newHeight = Height + e.VerticalChange;

        if (newWidth >= MinWidth) Width = newWidth;
        if (newHeight >= MinHeight) Height = newHeight;
    }

    private void NoteList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (e.AddedItems.Count > 0 && e.AddedItems[0] is Note note)
        {
            _vm.OpenExistingNoteCommand.Execute(note);
            NoteListBox.SelectedItem = null;
        }
    }

    private void OnContextMenuOpening(object sender, ContextMenuEventArgs e)
    {
        e.Handled = true;

        var menu = new ContextMenu();

        if (_vm.IsTile)
        {
            AddMenuItem(menu, "取消钉住", () => _vm.TogglePinCommand.Execute(null));
            AddMenuItem(menu, _vm.IsDesktopEmbedded ? "切回置顶悬浮" : "贴在桌面",
                () => _vm.ToggleDesktopEmbedCommand.Execute(null));
            if (!_vm.IsDesktopEmbedded)
            {
                AddMenuItem(menu, _vm.IsTopmost ? "取消置顶" : "置顶悬浮",
                    () => _vm.ToggleTopmostCommand.Execute(null));
            }
            AddMenuItem(menu, "在主窗口打开", () => _vm.SaveToNotesAndCloseCommand.Execute(null));
            menu.Items.Add(new Separator());
            AddMenuItem(menu, "关闭", () => _vm.RequestCloseCommand.Execute(null));
        }
        else
        {
            AddMenuItem(menu, "贴在一旁", () => _vm.TogglePinCommand.Execute(null));
            AddMenuItem(menu, "存入笔记", () => _vm.SaveToNotesAndCloseCommand.Execute(null));
            menu.Items.Add(new Separator());
            AddMenuItem(menu, "关闭", () => _vm.RequestCloseCommand.Execute(null));
        }

        menu.PlacementTarget = sender as UIElement;
        menu.Placement = PlacementMode.MousePoint;
        menu.IsOpen = true;
    }

    private static void AddMenuItem(ContextMenu menu, string header, Action action)
    {
        var item = new MenuItem { Header = header };
        item.Click += (_, _) => action();
        menu.Items.Add(item);
    }
}
