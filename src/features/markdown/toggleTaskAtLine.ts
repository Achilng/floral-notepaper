// 任务项源码行以列表标记 + 空格开头，复选框是该行第一个
// `[ ]`/`[x]`，直接替换行内第一个匹配即可，不受正文内容干扰
export function toggleTaskAtLine(source: string, lineNumber: number): string {
  const lines = source.split("\n");
  const line = lines[lineNumber - 1];
  if (line === undefined) return source;
  const toggled = line.includes("[ ]")
    ? line.replace("[ ]", "[x]")
    : line.replace(/\[[xX]\]/, "[ ]");
  if (toggled === line) return source;
  lines[lineNumber - 1] = toggled;
  return lines.join("\n");
}
