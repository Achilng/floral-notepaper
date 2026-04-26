using Markdig;

namespace 花笺.Services;

public static class MarkdownService
{
    private static readonly MarkdownPipeline Pipeline = new MarkdownPipelineBuilder()
        .UseAdvancedExtensions()
        .Build();

    public static string ToHtml(string markdown)
    {
        if (string.IsNullOrEmpty(markdown))
            return WrapInHtmlPage("<p style='color:#939F96;text-align:center;padding-top:40px;'>空内容</p>");

        var body = Markdown.ToHtml(markdown, Pipeline);
        return WrapInHtmlPage(body);
    }

    private static string WrapInHtmlPage(string body) => $$"""
        <!DOCTYPE html>
        <html>
        <head>
        <meta charset="utf-8" />
        <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Noto Serif SC', 'SimSun', serif;
            font-size: 13px;
            line-height: 1.9;
            color: #1E2D23;
            padding: 20px 24px;
            background: #F7FAF8;
            letter-spacing: 0.02em;
        }
        h1 { font-size: 18px; font-weight: 500; margin: 0 0 12px; border-bottom: 1px solid #D6DDD8; padding-bottom: 6px; }
        h2 { font-size: 15px; font-weight: 500; margin: 16px 0 8px; }
        h3 { font-size: 13px; font-weight: 500; margin: 12px 0 6px; }
        p { margin: 0 0 10px; }
        ul, ol { padding-left: 20px; margin: 0 0 10px; }
        li { margin: 2px 0; }
        code {
            font-family: Consolas, monospace;
            font-size: 11px;
            background: #EFF4F1;
            padding: 1px 4px;
            border-radius: 3px;
            color: #4A8A95;
        }
        pre {
            background: #EFF4F1;
            border: 1px solid #D6DDD8;
            border-radius: 6px;
            padding: 12px 14px;
            margin: 0 0 12px;
            overflow-x: auto;
        }
        pre code { background: none; padding: 0; color: #1E2D23; }
        blockquote {
            border-left: 2.5px solid #3A8A5C;
            padding-left: 12px;
            margin: 0 0 10px;
            color: #586B5E;
            font-style: italic;
        }
        strong { font-weight: 500; }
        em { color: #586B5E; }
        hr { border: none; border-top: 1px solid #D6DDD8; margin: 16px 0; }
        a { color: #3A8A5C; text-decoration: none; }
        a:hover { text-decoration: underline; }
        table { width: 100%; border-collapse: collapse; margin: 0 0 12px; font-size: 12px; }
        th, td { border: 1px solid #D6DDD8; padding: 5px 10px; text-align: left; }
        th { background: #EFF4F1; font-weight: 500; }
        img { max-width: 100%; border-radius: 4px; }
        ::-webkit-scrollbar { width: 4px; }
        ::-webkit-scrollbar-thumb { background: #D6DDD8; border-radius: 2px; }
        </style>
        </head>
        <body>{{body}}</body>
        </html>
        """;
}
