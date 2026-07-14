/** Syntax highlight palettes — independent of app light/dark mode. */

export type SyntaxThemeId = "one-dark" | "github-light" | "monokai";

export type SyntaxTokenStyle =
  | "plain"
  | "comment"
  | "keyword"
  | "string"
  | "function"
  | "type"
  | "number"
  | "punctuation";

export interface SyntaxColors {
  plain: string;
  comment: string;
  keyword: string;
  string: string;
  function: string;
  type: string;
  number: string;
  punctuation: string;
}

export interface SyntaxThemeOption {
  id: SyntaxThemeId;
  label: string;
  colors: SyntaxColors;
}

export const SYNTAX_THEME_OPTIONS: readonly SyntaxThemeOption[] = [
  {
    id: "one-dark",
    label: "One Dark",
    colors: {
      plain: "#abb2bf",
      comment: "#5c6370",
      keyword: "#c678dd",
      string: "#98c379",
      function: "#61afef",
      type: "#e5c07b",
      number: "#d19a66",
      punctuation: "#abb2bf",
    },
  },
  {
    id: "github-light",
    label: "GitHub Light",
    colors: {
      plain: "#24292f",
      comment: "#6e7781",
      keyword: "#cf222e",
      string: "#0a3069",
      function: "#8250df",
      type: "#953800",
      number: "#0550ae",
      punctuation: "#24292f",
    },
  },
  {
    id: "monokai",
    label: "Monokai",
    colors: {
      plain: "#f8f8f2",
      comment: "#75715e",
      keyword: "#f92672",
      string: "#e6db74",
      function: "#a6e22e",
      type: "#66d9ef",
      number: "#ae81ff",
      punctuation: "#f8f8f2",
    },
  },
] as const;

export function syntaxThemeById(id: SyntaxThemeId): SyntaxThemeOption {
  return SYNTAX_THEME_OPTIONS.find((entry) => entry.id === id) ?? SYNTAX_THEME_OPTIONS[0];
}
