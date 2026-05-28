import {
  StreamLanguage,
  type StreamParser,
  HighlightStyle,
  syntaxHighlighting,
  indentUnit,
  bracketMatching,
  indentOnInput,
} from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

/** Spaces per indent level. Tab and auto-indent both use this. */
export const INDENT_SIZE = 2;
const INDENT_UNIT = " ".repeat(INDENT_SIZE);

// Keyword groups, drawn from praxsmth.pest. Split so they can be highlighted
// with distinct colours.
const TYPE_KEYWORDS = new Set([
  "trait",
  "exclusive",
  "directional",
  "reciprocal",
  "evaluation",
  "emotion",
  "practice",
  "actor",
]);

const STRUCTURE_KEYWORDS = new Set([
  "actions",
  "name",
  "conditions",
  "outcomes",
  "goal",
  "delta",
  "inactive",
  "as",
]);

const EFFECT_KEYWORDS = new Set([
  "broadcast",
  "say",
  "activate",
  "deactivate",
  "delete",
  "create",
  "set",
  "increase",
  "cycle",
  "to",
  "by",
]);

// Expression operators / quantifiers / aggregations.
const OPERATOR_KEYWORDS = new Set([
  "and",
  "or",
  "is",
  "not",
  "for",
  "all",
  "any",
  "where",
  "count",
  "sum",
  "average",
  "min",
  "max",
  "across",
]);

interface State {
  /** Net bracket depth at the current stream position. */
  depth: number;
}

const parser: StreamParser<State> = {
  startState: () => ({ depth: 0 }),

  token(stream, state) {
    if (stream.eatSpace()) return null;

    // Line comments: // to end of line.
    if (stream.match("//")) {
      stream.skipToEnd();
      return "comment";
    }

    // Strings with escape handling.
    if (stream.match('"')) {
      let escaped = false;
      let ch: string | void;
      while ((ch = stream.next()) != null) {
        if (ch === '"' && !escaped) break;
        escaped = !escaped && ch === "\\";
      }
      return "string";
    }

    // Numbers (incl. negative and decimals).
    if (stream.match(/^-?\d+(\.\d+)?/)) return "number";

    // Track bracket depth for indentation.
    if (stream.match(/^[{([]/)) {
      state.depth++;
      return "bracket";
    }
    if (stream.match(/^[})\]]/)) {
      if (state.depth > 0) state.depth--;
      return "bracket";
    }

    // Identifiers / keywords.
    const word = stream.match(/^[A-Za-z_][A-Za-z0-9_]*/);
    if (word) {
      const w = (word as RegExpMatchArray)[0];
      if (TYPE_KEYWORDS.has(w)) return "keyword";
      if (EFFECT_KEYWORDS.has(w)) return "effect";
      if (OPERATOR_KEYWORDS.has(w)) return "operator";
      if (STRUCTURE_KEYWORDS.has(w)) return "structure";
      return "variable";
    }

    // Punctuation we don't otherwise classify.
    stream.next();
    return null;
  },

  indent(state, textAfter) {
    // A line beginning with a closing bracket dedents one level.
    let depth = state.depth;
    if (/^\s*[})\]]/.test(textAfter)) depth = Math.max(0, depth - 1);
    return depth * INDENT_SIZE;
  },

  languageData: {
    commentTokens: { line: "//" },
    closeBrackets: { brackets: ["(", "[", "{", '"'] },
  },
};

// Map the token names returned above onto highlight tags.
export const praxsmthLanguage = StreamLanguage.define({
  ...parser,
  token(stream, state) {
    const tok = parser.token(stream, state);
    switch (tok) {
      case "keyword":
        return "keyword";
      case "effect":
        return "macroName";
      case "operator":
        return "operatorKeyword";
      case "structure":
        return "labelName";
      case "variable":
        return "variableName";
      case "number":
        return "number";
      case "string":
        return "string";
      case "comment":
        return "comment";
      case "bracket":
        return "bracket";
      default:
        return null;
    }
  },
});

const highlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: "#9a3b2f", fontWeight: "600" },
  { tag: t.macroName, color: "#1f6f5c", fontWeight: "600" },
  { tag: t.operatorKeyword, color: "#7a5a1e" },
  { tag: t.labelName, color: "#6b4ea0" },
  { tag: t.variableName, color: "#2a2622" },
  { tag: t.number, color: "#2f6f9a" },
  { tag: t.string, color: "#3d7a2f" },
  { tag: t.comment, color: "#a39684", fontStyle: "italic" },
  { tag: t.bracket, color: "#8a7f6d" },
]);

/** The full set of language-related extensions for the DSL editor. */
export function praxsmthExtensions(): Extension[] {
  return [
    praxsmthLanguage,
    syntaxHighlighting(highlightStyle),
    indentUnit.of(INDENT_UNIT),
    bracketMatching(),
    indentOnInput(),
  ];
}
