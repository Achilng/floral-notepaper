const MARKDOWN_IMAGE_PATTERN = /!\[[^\]\r\n]*\]\([^\r\n)]+\)/;

export function shouldRenderTileMarkdown(content: string, renderMarkdown: boolean): boolean {
  return renderMarkdown || MARKDOWN_IMAGE_PATTERN.test(content);
}
