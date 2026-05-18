import { useMemo, useState, useCallback } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkFootnotes from "remark-footnotes";
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Components } from "react-markdown";
import type { PluggableList } from "unified";
import type { Schema } from "hast-util-sanitize";
import remarkHighlight from "./remarkHighlight";
import { useTranslation } from "../i18n/LanguageContext";

const sanitizeSchema: Schema = {
  ...defaultSchema,
  tagNames: [
    ...(defaultSchema.tagNames ?? []),
    "mark",
    "section",
    "sub",
    "sup",
  ],
  attributes: {
    ...defaultSchema.attributes,
    mark: ["className"],
    section: ["className", "id"],
    div: ["className"],
    span: ["className"],
    sup: ["id"],
    input: ["checked", "disabled", "type"],
    a: [...(defaultSchema.attributes?.a || []), "href"],
    img: [...(defaultSchema.attributes?.img || []), "src", "alt"],
  },
  protocols: {
    ...defaultSchema.protocols,
    href: ["http", "https", "mailto", "tel"],
    src: ["http", "https", "data"],
  },
};

function CodeBlock({ children }: { children: React.ReactNode }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    const text = extractText(children);
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [children]);

  return (
    <pre className="my-3 px-4 py-3 rounded bg-paper-warm/80 overflow-x-auto relative group">
      <button
        type="button"
        onClick={handleCopy}
        className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono bg-paper-deep/30 text-ink-ghost opacity-0 group-hover:opacity-100 hover:bg-paper-deep/50 hover:text-ink-soft transition-all cursor-pointer"
      >
        {copied ? "已复制" : "复制"}
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
  tileMode?: boolean;
  className?: string;
}

function useMarkdownComponents(tileMode = false): Components {
  const { t } = useTranslation();

  return useMemo(
    () => {
      if (tileMode) {
        return {
          h1: ({ children }) => (
            <h1 className="font-display font-bold mt-4 mb-2 tracking-wide" style={{ fontSize: "1.25em" }}>
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="font-display font-bold mt-3 mb-2 tracking-wide" style={{ fontSize: "1.1em" }}>
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="font-display font-bold mt-2 mb-1 tracking-wide" style={{ fontSize: "1.05em" }}>
              {children}
            </h3>
          ),
          h4: ({ children }) => (
            <h4 className="font-display font-semibold mt-2 mb-1 tracking-wide" style={{ fontSize: "1em" }}>
              {children}
            </h4>
          ),
          p: ({ children }) => (
            <p className="leading-[1.9]">{children}</p>
          ),
          strong: ({ children }) => (
            <strong className="font-semibold">{children}</strong>
          ),
          em: ({ children }) => (
            <em className="italic opacity-80">{children}</em>
          ),
          blockquote: ({ children }) => (
            <blockquote className="border-l-2 border-current/30 pl-3 my-2 opacity-80 italic leading-[1.9]">
              {children}
            </blockquote>
          ),
          ul: ({ children }) => (
            <ul className="ml-4 leading-[1.9] list-disc list-outside">
              {children}
            </ul>
          ),
          ol: ({ children }) => (
            <ol className="ml-4 leading-[1.9] list-decimal list-outside">
              {children}
            </ol>
          ),
          li: ({ children }) => (
            <li className="leading-[1.9]">{children}</li>
          ),
          hr: () => (
            <hr className="my-4 border-none h-px bg-current/15" />
          ),
          code: ({ className, children }) => {
            const isBlock =
              className?.startsWith("language-") ||
              String(children).includes("\n");
            if (isBlock) {
              return (
                <code className="font-mono leading-[1.8] whitespace-pre" style={{ fontSize: "0.85em" }}>
                  {children}
                </code>
              );
            }
            return (
              <code className="px-1 py-0.5 font-mono rounded bg-current/8" style={{ fontSize: "0.85em" }}>
                {children}
              </code>
            );
          },
          pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
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
              className="underline underline-offset-2 cursor-pointer opacity-75 hover:opacity-100"
            >
              {children}
            </a>
          ),
          table: ({ children }) => (
            <div className="my-2 overflow-x-auto">
              <table className="w-full border-collapse" style={{ fontSize: "0.9em" }}>
                {children}
              </table>
            </div>
          ),
          th: ({ children }) => (
            <th className="text-left px-2 py-1 border-b border-current/15 font-semibold">
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="px-2 py-1 border-b border-current/8">
              {children}
            </td>
          ),
          input: ({ checked, ...props }) => (
            <input
              {...props}
              checked={checked}
              disabled
              className="mr-1"
            />
          ),
          sup: ({ children, id }) => (
            <sup
              id={id}
              className="cursor-pointer hover:underline font-mono align-super"
              style={{ fontSize: "0.7em" }}
              onClick={() => document.getElementById(`fn-${id}`)?.scrollIntoView()}
            >
              {children}
            </sup>
          ),
          sub: ({ children }) => (
            <sub className="font-mono align-sub" style={{ fontSize: "0.7em" }}>
              {children}
            </sub>
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
                <section className="mt-4 pt-3 border-t border-current/15">
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
            <mark className="bg-yellow-200/60 px-0.5 rounded">{children}</mark>
          ),
        };
      }

      return {
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
        pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
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
            className="cursor-pointer hover:underline text-[10px] font-mono text-bamboo align-super"
            onClick={() => document.getElementById(`fn-${id}`)?.scrollIntoView()}
          >
            {children}
          </sup>
        ),
        sub: ({ children }) => (
          <sub className="text-[10px] font-mono text-ink-faint align-sub">
            {children}
          </sub>
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
          <mark className="bg-yellow-200 px-1 rounded">{children}</mark>
        ),
      };
    },
    [t, tileMode],
  );
}

export function MarkdownPreview({
  content,
  fontSize = 14,
  tileMode = false,
  className,
}: MarkdownPreviewProps) {
  const { t } = useTranslation();
  const components = useMarkdownComponents(tileMode);

  const remarkPlugins = useMemo(
    () => [remarkGfm, [remarkFootnotes, { inlineNotes: true }], remarkHighlight] as PluggableList,
    [],
  );

  const rehypePlugins = useMemo(
    () => [rehypeRaw, [rehypeSanitize, sanitizeSchema]] as PluggableList,
    [],
  );

  if (!content.trim()) {
    if (tileMode) return null;
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

  const rootClassName = tileMode
    ? `font-body markdown-preview ${className ?? ""}`
    : `max-w-[560px] font-body markdown-preview ${className ?? ""}`;

  return (
    <div
      className={rootClassName}
      style={{ fontSize: `${fontSize}px` }}
    >
      <Markdown
        remarkPlugins={remarkPlugins}
        rehypePlugins={rehypePlugins}
        components={components}
      >
        {content}
      </Markdown>
    </div>
  );
}
