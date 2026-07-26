import { Suspense, lazy } from "react";
import type { ComponentProps } from "react";

// react-markdown + unified 插件链 + KaTeX（含 CSS）是 bundle 中最大的一块，
// 懒加载后仅在真正渲染预览时才下载并解析，纯编辑视图和池中 standby 窗口不再付出这份内存
const MarkdownPreviewInner = lazy(() =>
  import("./MarkdownPreview").then((module) => ({ default: module.MarkdownPreview })),
);

export function MarkdownPreviewLazy(props: ComponentProps<typeof MarkdownPreviewInner>) {
  return (
    <Suspense fallback={null}>
      <MarkdownPreviewInner {...props} />
    </Suspense>
  );
}
