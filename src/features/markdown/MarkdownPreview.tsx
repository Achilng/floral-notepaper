import { useMemo } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFootnotes from "remark-footnotes";
import rehypeRaw from "rehype-raw";
import DOMPurify from "isomorphic-dompurify";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Components } from "react-markdown";
import type { PluggableList } from "unified";
import remarkHighlight from "./remarkHighlight";
import { useTranslation } from "../i18n/LanguageContext";

interface MarkdownPreviewProps {
  content: string;
  fontSize?: number;
}

function useMarkdownComponents(): Components {
  const { t } = useTranslation();

  return useMemo(
    () => ({
      h1: ({ children }) => (
        <h1 className="text-[22px] font-display font-bold text-ink mt-6 mb-4 tracking-wide">
          {children}
        </h1>
      ),
      h2: ({ children }) => (
        <h2 className="text-[17px] font-display font-bold text-ink mt-7 mb-3 tracking-wide">
          {children}
        </h2>
      ),
      h3: ({ children }) => (
        <h3 className="text-[15px] font-display font-bold text-ink mt-5 mb-2 tracking-wide">
          {children}
        </h3>
      ),
      h4: ({ children }) => (
        <h4 className="text-[14px] font-display font-semibold text-ink mt-4 mb-2 tracking-wide">
          {children}
        </h4>
      ),
      p: ({ children }) => (
        <p className="text-ink-soft leading-[1.9]">{children}</p>
      ),
      strong: ({ children }) => (
        <strong className="font-semibold text-ink">{children}</strong>
      ),
      em: ({ children }) => (
        <em className="italic text-bamboo-light">{children}</em>
      ),
      blockquote: ({ children }) => (
        <blockquote className="border-l-2 border-bamboo/40 pl-4 my-3 text-ink-soft/80 italic leading-[1.9]">
          {children}
        </blockquote>
      ),
      ul: ({ children }) => (
        <ul className="ml-4 text-ink-soft leading-[1.9] list-disc list-outside marker:text-bamboo/40">
          {children}
        </ul>
      ),
      ol: ({ children }) => (
        <ol className="ml-4 text-ink-soft leading-[1.9] list-decimal list-outside marker:text-bamboo/50 marker:font-mono marker:text-[12px]">
          {children}
        </ol>
      ),
      li: ({ children }) => (
        <li className="text-ink-soft leading-[1.9]">{children}</li>
      ),
      hr: () => (
        <hr className="my-6 border-none h-px bg-gradient-to-r from-transparent via-paper-deep to-transparent" />
      ),
      code: ({ className, children }) => {
        const isBlock =
          className?.startsWith("language-") ||
          String(children).includes("\n");
        if (isBlock) {
          return (
            <code className="text-[12px] font-mono text-ink-soft leading-[1.8] whitespace-pre">
              {children}
            </code>
          );
        }
        return (
          <code className="px-1.5 py-0.5 text-[12px] font-mono bg-paper-warm rounded text-bamboo">
            {children}
          </code>
        );
      },
      pre: ({ children }) => (
        <pre className="my-3 px-4 py-3 rounded bg-paper-warm/80 overflow-x-auto">
          {children}
        </pre>
      ),
      a: ({ href, children }) => (
        <a
          href={href}
          onClick={(e) => {
            e.preventDefault();
            if (href) {
              if (href.startsWith("#")) {
                const target = document.getElementById(href.slice(1));
                target?.scrollIntoView({ behavior: "smooth" });
              } else {
                openUrl(href);
              }
            }
          }}
          className="text-bamboo hover:text-bamboo-light underline underline-offset-2 cursor-pointer"
        >
          {children}
        </a>
      ),
      table: ({ children }) => (
        <div className="my-3 overflow-x-auto">
          <table className="w-full text-[13px] border-collapse">
            {children}
          </table>
        </div>
      ),
      th: ({ children }) => (
        <th className="text-left px-3 py-1.5 border-b border-paper-deep/30 font-semibold text-ink text-[12px]">
          {children}
        </th>
      ),
      td: ({ children }) => (
        <td className="px-3 py-1.5 border-b border-paper-deep/15 text-ink-soft">
          {children}
        </td>
      ),
      input: ({ checked, ...props }) => (
        <input
          {...props}
          checked={checked}
          disabled
          className="mr-1.5 accent-bamboo"
        />
      ),
      sup: ({ children, id }) => (
        <sup
          id={id}
          className="text-[10px] font-mono text-bamboo align-super"
        >
          {children}
        </sup>
      ),
      section: ({ children, ...props }) => {
        const className = props.className ?? props.node?.properties?.className;
        if (
          className &&
          (Array.isArray(className)
            ? className.includes("footnotes")
            : String(className).includes("footnotes"))
        ) {
          return (
            <section className="footnotes mt-6 pt-4 border-t border-paper-deep/25">
              {children}
            </section>
          );
        }
        return <section>{children}</section>;
      },
      div: ({ children, className }) => (
        <div className={className}>{children}</div>
      ),
      span: ({ children, className }) => (
        <span className={className}>{children}</span>
      ),
      img: ({ src, alt }) => (
        <img
          src={src}
          alt={alt ?? ""}
          className="max-w-full rounded my-2"
          loading="lazy"
        />
      ),
      mark: ({ children }) => (
        <mark className="bg-bamboo-mist/80 text-ink rounded-sm px-0.5">
          {children}
        </mark>
      ),
    }),
    [t],
  );
}

const ALLOWED_TAGS = [
  "div",
  "span",
  "img",
  "a",
  "code",
  "pre",
  "mark",
  "sub",
  "sup",
  "abbr",
  "details",
  "summary",
  "kbd",
  "samp",
  "var",
  "del",
  "ins",
  "br",
  "hr",
  "wbr",
  "b",
  "i",
  "u",
  "small",
  "ruby",
  "rt",
  "rp",
  "bdi",
  "bdo",
];

function sanitizeContent(content: string): string {
  return DOMPurify.sanitize(content, {
    ALLOWED_TAGS,
    ALLOWED_ATTR: [
      "class",
      "id",
      "href",
      "src",
      "alt",
      "title",
      "target",
      "rel",
      "colspan",
      "rowspan",
      "align",
      "valign",
      "width",
      "height",
      "loading",
      "start",
      "reversed",
      "type",
      "name",
      "value",
      "checked",
      "disabled",
      "for",
      "data-*",
    ],
    ALLOW_DATA_ATTR: true,
  });
}

export function MarkdownPreview({
  content,
  fontSize = 14,
}: MarkdownPreviewProps) {
  const { t } = useTranslation();
  const components = useMarkdownComponents();

  const remarkPlugins = useMemo(
    () => [remarkGfm, remarkFootnotes, remarkHighlight] as PluggableList,
    [],
  );

  const rehypePlugins = useMemo(() => [rehypeRaw] as PluggableList, []);

  const sanitizedContent = useMemo(() => {
    const preprocessed = content.replace(/==([^=]+)==/g, (match) => match);
    return sanitizeContent(preprocessed);
  }, [content]);

  if (!content.trim()) {
    return (
      <div
        className="max-w-[560px] font-body"
        style={{ fontSize: `${fontSize}px` }}
      >
        <p className="text-ink-ghost leading-[1.9]">
          {t("main.previewPlaceholder")}
        </p>
      </div>
    );
  }

  return (
    <div
      className="max-w-[560px] font-body markdown-preview"
      style={{ fontSize: `${fontSize}px` }}
    >
      <Markdown
        remarkPlugins={remarkPlugins}
        rehypePlugins={rehypePlugins}
        components={components}
      >
        {sanitizedContent}
      </Markdown>
    </div>
  );
}
