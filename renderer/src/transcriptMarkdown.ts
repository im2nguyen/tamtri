export function normalizedTranscriptMarkdown(content: string): string {
  let text = sanitizedMarkdownForPreview(content).replace(/\r\n/g, "\n");

  text = text.replace(/(?<![\n\r])(\n)(?=[\-*+]\s|\d+\.\s)/g, "\n\n");

  const paragraphs = text.split("\n\n");
  const normalized = paragraphs.map((paragraph) => {
    const lines = paragraph.split("\n");
    if (lines.length <= 1) return paragraph;

    const isListBlock = lines.some((line) => {
      const trimmed = line.trim();
      if (!trimmed) return false;
      return (
        trimmed.startsWith("- ") ||
        trimmed.startsWith("* ") ||
        trimmed.startsWith("+ ") ||
        /^\d+\.\s/.test(trimmed)
      );
    });

    if (isListBlock) return paragraph;
    return lines.join("  \n");
  });

  return normalized.join("\n\n");
}

function sanitizedMarkdownForPreview(content: string): string {
  let sanitized = content;
  const dangerousBlockPatterns = [
    /<script\b[^>]*>[\s\S]*?<\/script>/gi,
    /<style\b[^>]*>[\s\S]*?<\/style>/gi,
    /<iframe\b[^>]*>[\s\S]*?<\/iframe>/gi,
    /<object\b[^>]*>[\s\S]*?<\/object>/gi,
    /<embed\b[^>]*>/gi,
  ];
  for (const pattern of dangerousBlockPatterns) {
    sanitized = sanitized.replace(pattern, "");
  }
  sanitized = sanitized.replace(/<[^>]+>/g, "");
  return sanitized;
}
