import zhCN from "./zh-CN/tips.json";
import enUS from "./en-US/tips.json";
import zhHK from "./zh-HK/tips.json";

const tipsMap: Record<string, string[]> = {
  "zh-CN": zhCN,
  "en-US": enUS,
  "zh-HK": zhHK,
};

export function getTips(language: string): string[] {
  return tipsMap[language] ?? tipsMap["zh-CN"];
}

export interface TipSegment {
  type: "text" | "link";
  text: string;
  url?: string;
}

const LINK_RE = /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g;

export function parseTip(tip: string): TipSegment[] {
  const segments: TipSegment[] = [];
  let lastIndex = 0;

  for (const match of tip.matchAll(LINK_RE)) {
    const matchIndex = match.index!;
    if (matchIndex > lastIndex) {
      segments.push({ type: "text", text: tip.slice(lastIndex, matchIndex) });
    }
    segments.push({ type: "link", text: match[1], url: match[2] });
    lastIndex = matchIndex + match[0].length;
  }

  if (lastIndex < tip.length) {
    segments.push({ type: "text", text: tip.slice(lastIndex) });
  }

  return segments;
}
