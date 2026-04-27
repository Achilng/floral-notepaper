import type { ReactNode } from "react";

interface MarkdownPreviewProps {
  content: string;
}

export function MarkdownPreview({ content }: MarkdownPreviewProps) {
  return (
    <div className="max-w-[560px] text-[14px] font-body">
      {content.trim() ? renderMarkdownPreview(content) : (
        <p className="text-ink-ghost leading-[1.9]">预览区会显示当前笔记内容</p>
      )}
    </div>
  );
}

function renderMarkdownPreview(markdown: string): ReactNode[] {
  return markdown.split("\n").map((line, index) => {
    if (line.startsWith("## ")) {
      return (
        <h2
          key={index}
          className="text-[17px] font-display font-bold text-ink mt-7 mb-3 tracking-wide"
        >
          {line.slice(3)}
        </h2>
      );
    }

    if (line.startsWith("# ")) {
      return (
        <h1
          key={index}
          className="text-[22px] font-display font-bold text-ink mt-6 mb-4 tracking-wide"
        >
          {line.slice(2)}
        </h1>
      );
    }

    if (line.startsWith("> ")) {
      return (
        <blockquote
          key={index}
          className="border-l-2 border-bamboo/40 pl-4 my-3 text-ink-soft/80 italic leading-[1.9]"
        >
          {renderInlineMarkdown(line.slice(2))}
        </blockquote>
      );
    }

    if (line.startsWith("- ")) {
      return (
        <li
          key={index}
          className="ml-4 text-ink-soft leading-[1.9] list-disc list-outside marker:text-bamboo/40"
        >
          {renderInlineMarkdown(line.slice(2))}
        </li>
      );
    }

    if (/^\d+\.\s/.test(line)) {
      return (
        <li
          key={index}
          className="ml-4 text-ink-soft leading-[1.9] list-decimal list-outside marker:text-bamboo/50 marker:font-mono marker:text-[12px]"
        >
          {renderInlineMarkdown(line.replace(/^\d+\.\s/, ""))}
        </li>
      );
    }

    if (line === "---") {
      return (
        <hr
          key={index}
          className="my-6 border-none h-px bg-gradient-to-r from-transparent via-paper-deep to-transparent"
        />
      );
    }

    if (line.trim() === "") {
      return <div key={index} className="h-3" />;
    }

    return (
      <p key={index} className="text-ink-soft leading-[1.9]">
        {renderInlineMarkdown(line)}
      </p>
    );
  });
}

function renderInlineMarkdown(text: string): ReactNode[] {
  const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`|「[^」]+」)/g);
  return parts.map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**")) {
      return (
        <strong key={index} className="font-semibold text-ink">
          {part.slice(2, -2)}
        </strong>
      );
    }

    if (part.startsWith("*") && part.endsWith("*")) {
      return (
        <em key={index} className="italic text-bamboo-light">
          {part.slice(1, -1)}
        </em>
      );
    }

    if (part.startsWith("`") && part.endsWith("`")) {
      return (
        <code
          key={index}
          className="px-1.5 py-0.5 text-[12px] font-mono bg-paper-warm rounded text-bamboo"
        >
          {part.slice(1, -1)}
        </code>
      );
    }

    if (part.startsWith("「") && part.endsWith("」")) {
      return (
        <span key={index} className="text-ink font-display">
          {part}
        </span>
      );
    }

    return <span key={index}>{part}</span>;
  });
}
