// 工具栏“高亮”动作的纯文本改写逻辑：在选区上切换 `==…==` 标记——
// 已被标记包裹时移除标记，否则用标记包裹选区。
export interface HighlightEdit {
  result: string;
  cursorStart: number;
  cursorEnd: number;
}

export function toggleHighlightSelection(
  value: string,
  start: number,
  end: number,
  fallback: string,
): HighlightEdit {
  const selected = value.slice(start, end);
  const before = value.slice(0, start);
  const after = value.slice(end);

  // 选区整体就是 ==内容==：直接去掉标记
  if (selected.length >= 4 && selected.startsWith("==") && selected.endsWith("==")) {
    const content = selected.slice(2, -2);
    if (content.trim() && !content.includes("==")) {
      return {
        result: before + content + after,
        cursorStart: start,
        cursorEnd: start + content.length,
      };
    }
  }

  // 选区两侧已经紧贴 ==：一并去掉
  if (selected.length > 0 && before.endsWith("==") && after.startsWith("==")) {
    return {
      result: before.slice(0, -2) + selected + after.slice(2),
      cursorStart: start - 2,
      cursorEnd: end - 2,
    };
  }

  // 其余情况：用标记包裹选区；没有选区时插入示例文本便于继续输入
  const content = selected || fallback;
  return {
    result: `${before}==${content}==${after}`,
    cursorStart: start + 2,
    cursorEnd: start + 2 + content.length,
  };
}
