import type { SyntaxTokenStyle } from "@/styles/syntax-themes";

export interface HighlightToken {
  text: string;
  style: SyntaxTokenStyle;
}

const KEYWORDS = new Set([
  "export",
  "function",
  "const",
  "return",
  "number",
  "string",
  "let",
  "var",
  "if",
  "else",
  "type",
  "interface",
]);

function pushPlain(tokens: HighlightToken[], text: string) {
  if (text.length > 0) tokens.push({ text, style: "plain" });
}

/** Lightweight regex tokenizer for the appearance preview snippet. */
export function tokenizeTypescriptLine(line: string): HighlightToken[] {
  const tokens: HighlightToken[] = [];
  let index = 0;

  while (index < line.length) {
    const rest = line.slice(index);

    if (rest.startsWith("//")) {
      tokens.push({ text: rest, style: "comment" });
      break;
    }

    const stringMatch = rest.match(/^(`(?:\\.|[^`\\])*`|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')/);
    if (stringMatch) {
      tokens.push({ text: stringMatch[0], style: "string" });
      index += stringMatch[0].length;
      continue;
    }

    const wordMatch = rest.match(/^[A-Za-z_$][\w$]*/);
    if (wordMatch) {
      const word = wordMatch[0];
      if (KEYWORDS.has(word)) {
        tokens.push({ text: word, style: "keyword" });
      } else if (/^[A-Z]/.test(word)) {
        tokens.push({ text: word, style: "type" });
      } else if (tokens.length > 0 && tokens[tokens.length - 1]?.text === "function") {
        tokens.push({ text: word, style: "function" });
      } else {
        pushPlain(tokens, word);
      }
      index += word.length;
      continue;
    }

    const numberMatch = rest.match(/^\d+/);
    if (numberMatch) {
      tokens.push({ text: numberMatch[0], style: "number" });
      index += numberMatch[0].length;
      continue;
    }

    const punctMatch = rest.match(/^[{}()[\];:.,<>+=\-/*&|!?]+/);
    if (punctMatch) {
      tokens.push({ text: punctMatch[0], style: "punctuation" });
      index += punctMatch[0].length;
      continue;
    }

    pushPlain(tokens, rest[0] ?? "");
    index += 1;
  }

  return tokens;
}

export function tokenizeLines(lines: string[]): HighlightToken[][] {
  return lines.map((line) => tokenizeTypescriptLine(line));
}
