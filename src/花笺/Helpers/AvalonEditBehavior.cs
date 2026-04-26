using System.Windows;
using ICSharpCode.AvalonEdit;

namespace 花笺.Helpers;

public static class AvalonEditBehavior
{
    private static readonly DependencyProperty IsHandlerAttachedProperty =
        DependencyProperty.RegisterAttached(
            "IsHandlerAttached",
            typeof(bool),
            typeof(AvalonEditBehavior),
            new PropertyMetadata(false));

    public static readonly DependencyProperty BoundTextProperty =
        DependencyProperty.RegisterAttached(
            "BoundText",
            typeof(string),
            typeof(AvalonEditBehavior),
            new FrameworkPropertyMetadata(string.Empty, FrameworkPropertyMetadataOptions.BindsTwoWayByDefault, OnBoundTextChanged));

    public static string GetBoundText(DependencyObject obj) =>
        (string)obj.GetValue(BoundTextProperty);

    public static void SetBoundText(DependencyObject obj, string value) =>
        obj.SetValue(BoundTextProperty, value);

    private static bool _suppressUpdate;

    private static void OnBoundTextChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not TextEditor editor) return;
        if (_suppressUpdate) return;

        var newText = e.NewValue as string ?? string.Empty;
        if (editor.Document.Text != newText)
        {
            _suppressUpdate = true;
            editor.Document.Text = newText;
            _suppressUpdate = false;
        }

        if (!(bool)editor.GetValue(IsHandlerAttachedProperty))
        {
            editor.SetValue(IsHandlerAttachedProperty, true);
            editor.TextChanged += (_, _) =>
            {
                if (_suppressUpdate) return;
                _suppressUpdate = true;
                SetBoundText(editor, editor.Document.Text);
                _suppressUpdate = false;
            };
        }
    }
}
