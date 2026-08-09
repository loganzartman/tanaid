import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

/**
 * A CodeMirror theme that renders GNU Emacs 21.3's `font-lock` palette in both
 * light and dark at once, letting the browser pick with `light-dark()`.
 *
 * Colors are the `defface` specs from emacs-21.3 lisp/font-lock.el (syntax),
 * lisp/faces.el (region, fringe) and lisp/paren.el (paren match), taking the
 * `((class color) (background light))` and `(background dark)` branches.
 * Emacs names X11 colors, so each is resolved to its hex from X11 rgb.txt and
 * the original name is kept in the comment.
 *
 * This palette is remarkably stable across the era: between Emacs 19.34 (1996)
 * and 21.3 (2003) only three colors moved, so it is representative of the whole
 * period rather than of one release.
 */
const ld = (light: string, dark: string) => `light-dark(${light}, ${dark})`;

const c = {
  /** default face — Emacs frames default to white-on-black / black-on-white */
  background: ld("#FFFFFF", "#000000"),
  foreground: ld("#000000", "#FFFFFF"),

  /** font-lock-comment-face — Firebrick / chocolate1 */
  comment: ld("#B22222", "#FF7F24"),
  /** font-lock-string-face — RosyBrown / LightSalmon */
  string: ld("#BC8F8F", "#FFA07A"),
  /** font-lock-constant-face — CadetBlue / Aquamarine */
  constant: ld("#5F9EA0", "#7FFFD4"),
  /** font-lock-keyword-face — Purple / Cyan */
  keyword: ld("#A020F0", "#00FFFF"),
  /** font-lock-builtin-face — Orchid / LightSteelBlue */
  builtin: ld("#DA70D6", "#B0C4DE"),
  /** font-lock-variable-name-face — DarkGoldenrod / LightGoldenrod */
  variable: ld("#B8860B", "#EEDD82"),
  /** font-lock-function-name-face — Blue / LightSkyBlue */
  function: ld("#0000FF", "#87CEFA"),
  /** font-lock-type-face — ForestGreen / PaleGreen */
  type: ld("#228B22", "#98FB98"),

  // Chrome colors.
  /** region — LightGoldenrod2 / blue3 */
  selection: ld("#EEDC82", "#0000CD"),
  /**
   * `cursor` is an empty defface; the caret takes the frame's foreground.
   */
  cursor: ld("#000000", "#FFFFFF"),
  /** fringe — grey95 / grey10 */
  gutterBackground: ld("#F2F2F2", "#1A1A1A"),
  /**
   * Emacs 21 has no line-number margin at all — linum.el arrives in 22 — so
   * there is no period source for these. Borrowed from the `shadow` face that
   * Emacs 22 introduced alongside it: grey50 / grey70.
   */
  lineNumber: ld("#7F7F7F", "#B3B3B3"),
  /** Likewise invented; linum-mode drew every number identically. */
  indentGuide: ld("#D9D9D9", "#333333"),
  /**
   * show-paren-match-face. The defface matches on `(class color)` alone, so
   * turquoise is used on both backgrounds.
   */
  matchingBracket: "#40E0D0",
};

const emacsHighlightStyle = HighlightStyle.define([
  // LineComment
  { tag: t.lineComment, color: c.comment },

  // QuotedString, Block, SetValue
  { tag: t.string, color: c.string },
  // CommandSub `[...]`, PackageName. Emacs has no interpolation face; type is
  // the closest unused slot and reads as "a value spliced in here".
  { tag: t.special(t.string), color: c.type },

  // Number, PackageVersion
  { tag: t.number, color: c.constant },

  // Tcl's structural words get keyword-face; its builtin command library gets
  // builtin-face. That split mirrors how Emacs' own tcl.el assigns the two.
  // if/else/while/for/foreach/switch/return/break/continue/catch/try/throw
  { tag: t.controlKeyword, color: c.keyword },
  // proc, set, package
  { tag: t.definitionKeyword, color: c.keyword },
  // global, variable
  { tag: t.modifier, color: c.keyword },
  // the per-command keywords (list/string/dict/io/...)
  { tag: t.keyword, color: c.builtin },
  // expr
  { tag: t.operatorKeyword, color: c.builtin },
  // eq/ne/lt/gt/le/ge
  { tag: t.operator, color: c.foreground },

  // Variable, VariableName
  { tag: t.variableName, color: c.variable },
  // VarName (the target of a `set`)
  { tag: t.definition(t.variableName), color: c.variable },
  // the `$` sigil
  { tag: t.special(t.variableName), color: c.variable },

  // ProcInvocationName
  { tag: t.function(t.name), color: c.function },
  // ProcName (the name in a `proc` definition)
  { tag: t.definition(t.function(t.name)), color: c.function },

  // SimpleWord, and `{}` / `[]` delimiters
  { tag: t.name, color: c.foreground },
  { tag: [t.brace, t.squareBracket], color: c.foreground },
]);

const emacsEditorTheme = EditorView.theme({
  // Required: `light-dark()` resolves against the element's used color scheme.
  "&": {
    colorScheme: "light dark",
    backgroundColor: c.background,
    color: c.foreground,
  },
  ".cm-scroller": {
    lineHeight: "calc(var(--us) * 22px)",
  },
  ".cm-content": {
    caretColor: c.cursor,
    fontFamily: "cour-16",
    fontSize: "calc(var(--us) * 16px)",
    padding: "calc(var(--us) * 4px) 0",
  },
  ".cm-line": {
    padding: "0 calc(var(--us) * 2px) 0 calc(var(--us) * 6px)",
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: c.cursor,
    borderLeftWidth: "calc(var(--us) * 1px)",
    marginLeft: "0",
  },
  // Emacs keeps the region the same color when the frame loses focus, so the
  // unfocused selection deliberately matches the focused one.
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    {
      backgroundColor: c.selection,
    },
  ".cm-selectionLayer .cm-selectionBackground": {
    backgroundColor: c.selection,
  },
  ".cm-gutters": {
    backgroundColor: c.gutterBackground,
    color: c.lineNumber,
    border: "none",
    fontFamily: "cour-16",
    fontSize: "calc(var(--us) * 16px)",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    paddingLeft: "calc(var(--us) * 5px)",
    paddingRight: "calc(var(--us) * 3px)",
    minWidth: "calc(var(--us) * 20px)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: c.lineNumber,
  },
  ".cm-activeLine": {
    backgroundColor: "transparent",
  },
  "&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket": {
    backgroundColor: c.matchingBracket,
    color: "#000000",
  },
  ".cm-indentGuide": {
    borderLeft: `calc(var(--us) * 1px) solid ${c.indentGuide}`,
  },
});

export const emacsTheme = (): Extension => [
  emacsEditorTheme,
  syntaxHighlighting(emacsHighlightStyle),
];
