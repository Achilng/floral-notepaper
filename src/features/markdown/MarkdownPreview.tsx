import { Info, Lightbulb, MessageSquareWarning, OctagonAlert, TriangleAlert } from "lucide-react";
import { useState, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import rehypeSlug from "rehype-slug";
import { openUrl } from "@tauri-apps/plugin-opener";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Components } from "react-markdown";
import "katex/dist/katex.min.css";
import remarkAlerts from "./remarkAlerts";

function CodeBlock({ children, language }: { children: React.ReactNode; language?: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    const text = extractText(children);
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [children]);

  return (
    <pre
      className={`my-3 px-4 rounded bg-paper-warm/80 overflow-x-auto relative group ${
        language ? "pt-8 pb-3" : "py-3"
      }`}
    >
      {language && (
        <span className="absolute top-2 left-3 text-[10px] font-mono text-ink-faint/70 uppercase tracking-wider select-none">
          {language}
        </span>
      )}
      <button
        type="button"
        onClick={handleCopy}
        className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono bg-paper-deep/30 text-ink-ghost opacity-0 group-hover:opacity-100 hover:bg-paper-deep/50 hover:text-ink-soft transition-all cursor-pointer"
      >
        {copied
          ? t("markdown.copied", { defaultValue: "已复制" })
          : t("markdown.copy", { defaultValue: "复制" })}
      </button>
      {children}
    </pre>
  );
}

function extractText(node: React.ReactNode): string {
  if (typeof node === "string") return node;
  if (typeof node === "number") return String(node);
  if (node == null || typeof node === "boolean") return "";
  if (Array.isArray(node)) return node.map(extractText).join("");
  if (typeof node === "object" && "props" in node) {
    return extractText((node as React.ReactElement<{ children?: React.ReactNode }>).props.children);
  }
  return "";
}

interface MarkdownPreviewProps {
  content: string;
  fontSize?: number;
  renderHtml?: boolean;
  imageBaseDir?: string;
}

const remarkPlugins = [remarkGfm, remarkMath, remarkAlerts];
const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [...(defaultSchema.tagNames ?? []), "mark", "center", "font", "u", "abbr"],
  attributes: {
    ...defaultSchema.attributes,
    "*": [
      ...(defaultSchema.attributes?.["*"] ?? []),
      "style",
      "className",
      "data-alert-type",
      "dataAlertType",
    ],
    font: ["color", "size", "face"],
    abbr: ["title"],
  },
};
const rehypePluginsDefault = [rehypeKatex, rehypeSlug];
const rehypePluginsWithHtml = [
  rehypeRaw,
  [rehypeSanitize, sanitizeSchema],
  rehypeKatex,
  rehypeSlug,
] as Parameters<typeof Markdown>[0]["rehypePlugins"];

function AlertIcon({ type }: { type: string }) {
  const iconProps = { size: 16 };
  switch (type) {
    case "note":
      return <Info {...iconProps} />;
    case "tip":
      return <Lightbulb {...iconProps} />;
    case "important":
      return <MessageSquareWarning {...iconProps} />;
    case "warning":
      return <TriangleAlert {...iconProps} />;
    case "caution":
      return <OctagonAlert {...iconProps} />;
    default:
      return null;
  }
}

function Blockquote({
  children,
  node,
}: {
  children?: React.ReactNode;
  node?: { properties?: Record<string, unknown> };
}) {
  const { t } = useTranslation();
  const alertType =
    ((node?.properties?.dataAlertType || node?.properties?.["data-alert-type"]) as string) ?? "";
  if (alertType) {
    const alertTitleMap: Record<string, string> = {
      note: t("markdown.alert.note", { defaultValue: "备注" }),
      tip: t("markdown.alert.tip", { defaultValue: "提示" }),
      important: t("markdown.alert.important", { defaultValue: "重要" }),
      warning: t("markdown.alert.warning", { defaultValue: "警告" }),
      caution: t("markdown.alert.caution", { defaultValue: "注意" }),
    };

    return (
      <div className={`markdown-alert markdown-alert-${alertType}`} role="note">
        <p className="markdown-alert-title">
          <AlertIcon type={alertType} />
          {alertTitleMap[alertType] ?? alertType.toUpperCase()}
        </p>
        {children}
      </div>
    );
  }
  return (
    <blockquote className="border-l-2 border-bamboo/40 pl-4 my-3 text-ink-soft/80 italic leading-[1.9]">
      {children}
    </blockquote>
  );
}

const staticComponents: Components = {
  h1: ({ children, id }) => (
    <h1 id={id} className="text-[1.57em] font-display font-bold text-ink mt-6 mb-4 tracking-wide">
      {children}
    </h1>
  ),
  h2: ({ children, id }) => (
    <h2 id={id} className="text-[1.21em] font-display font-bold text-ink mt-7 mb-3 tracking-wide">
      {children}
    </h2>
  ),
  h3: ({ children, id }) => (
    <h3 id={id} className="text-[1.07em] font-display font-bold text-ink mt-5 mb-2 tracking-wide">
      {children}
    </h3>
  ),
  h4: ({ children, id }) => (
    <h4 id={id} className="text-[1em] font-display font-semibold text-ink mt-4 mb-2 tracking-wide">
      {children}
    </h4>
  ),
  p: ({ children }) => <p className="text-ink-soft leading-[1.9]">{children}</p>,
  strong: ({ children }) => <strong className="font-semibold text-ink">{children}</strong>,
  em: ({ children }) => <em className="italic text-bamboo-light">{children}</em>,
  blockquote: Blockquote,
  ul: ({ children }) => (
    <ul className="ml-4 text-ink-soft leading-[1.9] list-disc list-outside marker:text-bamboo/40">
      {children}
    </ul>
  ),
  ol: ({ children }) => (
    <ol className="ml-4 text-ink-soft leading-[1.9] list-decimal list-outside marker:text-bamboo/50 marker:font-mono marker:text-[0.85em]">
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="text-ink-soft leading-[1.9]">{children}</li>,
  hr: () => (
    <hr className="my-6 border-none h-px bg-gradient-to-r from-transparent via-paper-deep to-transparent" />
  ),
  code: ({ className, children }) => {
    const isBlock = className?.startsWith("language-") || String(children).includes("\n");
    if (isBlock) {
      return (
        <code className="text-[0.85em] font-mono text-ink-soft leading-[1.8] whitespace-pre">
          {children}
        </code>
      );
    }
    return (
      <code className="px-1.5 py-0.5 text-[0.85em] font-mono bg-paper-warm rounded text-bamboo">
        {children}
      </code>
    );
  },
  pre: ({ children }) => {
    // Extract language from the <code> element's className
    let language = "";
    if (
      children != null &&
      typeof children === "object" &&
      "props" in (children as React.ReactElement)
    ) {
      const codeProps = (children as React.ReactElement<{ className?: string }>).props;
      const match = codeProps.className?.match(/language-(\S+)/);
      if (match) language = match[1];
    }

    return <CodeBlock language={language}>{children}</CodeBlock>;
  },
  a: ({ href, children }) => (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        if (!href) return;
        if (/^https?:\/\//i.test(href)) {
          openUrl(href);
        } else if (href.startsWith("#")) {
          const id = decodeURIComponent(href.slice(1));
          document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
        }
      }}
      className="text-bamboo hover:text-bamboo-light underline underline-offset-2 cursor-pointer"
    >
      {children}
    </a>
  ),
  table: ({ children }) => (
    <div className="my-3 overflow-x-auto">
      <table className="w-full text-[0.93em] border-collapse border border-paper-deep/50">
        {children}
      </table>
    </div>
  ),
  th: ({ children }) => (
    <th className="text-left px-3 py-1.5 border border-paper-deep/40 font-semibold text-ink text-[0.85em] bg-paper-warm/50">
      {children}
    </th>
  ),
  td: ({ children }) => (
    <td className="px-3 py-1.5 border border-paper-deep/35 text-ink-soft">{children}</td>
  ),
  input: ({ checked, ...props }) => (
    <input {...props} checked={checked} disabled className="mr-1.5 accent-bamboo" />
  ),
};

export function MarkdownPreview({
  content,
  fontSize = 14,
  renderHtml = false,
  imageBaseDir,
}: MarkdownPreviewProps) {
  const { t } = useTranslation();
  const components = useMemo<Components>(
    () => ({
      ...staticComponents,
      img: ({ src, alt, ...props }) => {
        let resolvedSrc = src ?? "";
        if (src?.startsWith("images/") && imageBaseDir) {
          resolvedSrc = convertFileSrc(imageBaseDir + "/" + src);
        }
        return (
          <img
            src={resolvedSrc}
            alt={alt ?? ""}
            loading="lazy"
            className="w-[50%] rounded my-2 mx-auto block"
            {...props}
          />
        );
      },
    }),
    [imageBaseDir],
  );
  return (
    <div className="font-body markdown-selectable" style={{ fontSize: `${fontSize}px` }}>
      {content.trim() ? (
        <Markdown
          remarkPlugins={remarkPlugins}
          rehypePlugins={renderHtml ? rehypePluginsWithHtml : rehypePluginsDefault}
          components={components}
        >
          {content}
        </Markdown>
      ) : (
        <p className="text-ink-ghost leading-[1.9]">
          {t("markdown.emptyHint", { defaultValue: "预览区会显示当前笔记内容" })}
        </p>
      )}
    </div>
  );
}
