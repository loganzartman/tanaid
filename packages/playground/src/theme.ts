import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

/**
 * A CodeMirror theme that renders VS Code's Light+ and Dark+ palettes at the
 * same time, letting the browser pick between them with `light-dark()`.
 *
 * Colors are lifted from vscode/extensions/theme-defaults/themes:
 * light_vs.json + light_plus.json and dark_vs.json + dark_plus.json.
 * Only the TextMate scopes that the Tcl grammar's tags map onto are covered.
 */
const ld = (light: string, dark: string) => `light-dark(${light}, ${dark})`;

const c = {
  /** editor.background */
  background: ld("#FFFFFF", "#1E1E1E"),
  /** editor.foreground — also `keyword.operator` and unstyled punctuation */
  foreground: ld("#000000", "#D4D4D4"),
  /** comment */
  comment: ld("#008000", "#6A9955"),
  /** string */
  string: ld("#a31515", "#ce9178"),
  /** constant.numeric */
  number: ld("#098658", "#b5cea8"),
  /** keyword, storage, storage.modifier, keyword.operator.wordlike */
  keyword: ld("#0000ff", "#569cd6"),
  /** keyword.control */
  controlKeyword: ld("#AF00DB", "#C586C0"),
  /** variable, meta.definition.variable.name */
  variable: ld("#001080", "#9CDCFE"),
  /** entity.name.function */
  function: ld("#795E26", "#DCDCAA"),
  /** punctuation.section.embedded — i.e. interpolation delimiters */
  embedded: ld("#0000ff", "#569cd6"),

  // Chrome colors. The +/vs themes don't override VS Code's built-in defaults
  // for these, so these are the registry defaults.
  /** editor.selectionBackground */
  selection: ld("#ADD6FF", "#264F78"),
  /** editor.inactiveSelectionBackground */
  inactiveSelection: ld("#E5EBF1", "#3A3D41"),
  /** editorCursor.foreground */
  cursor: ld("#000000", "#AEAFAD"),
  /** editorLineNumber.foreground */
  lineNumber: ld("#237893", "#858585"),
  /** editorLineNumber.activeForeground */
  activeLineNumber: ld("#0B216F", "#C6C6C6"),
  /** editorIndentGuide.background1 */
  indentGuide: ld("#D3D3D3", "#404040"),
  /** editorBracketMatch.background / .border */
  matchingBracket: ld("#0064001a", "#0064001a"),
  matchingBracketBorder: ld("#B9B9B9", "#888888"),
};

const vscodeHighlightStyle = HighlightStyle.define([
  // LineComment
  { tag: t.lineComment, color: c.comment },

  // QuotedString, Block, SetValue
  { tag: t.string, color: c.string },
  // CommandSub `[...]`, PackageName
  { tag: t.special(t.string), color: c.embedded },

  // Number, PackageVersion
  { tag: t.number, color: c.number },

  // TclKeyword and the per-command keywords (list/string/dict/io/...)
  { tag: t.keyword, color: c.keyword },
  // proc, set, package
  { tag: t.definitionKeyword, color: c.keyword },
  // global, variable
  { tag: t.modifier, color: c.keyword },
  // expr
  { tag: t.operatorKeyword, color: c.keyword },
  // if/else/while/for/foreach/switch/return/break/continue/catch/try/throw
  { tag: t.controlKeyword, color: c.controlKeyword },
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

const vscodeEditorTheme = EditorView.theme({
  // Required: `light-dark()` resolves against the element's used color scheme.
  "&": {
    colorScheme: "light dark",
    backgroundColor: c.background,
    color: c.foreground,
  },
  ".cm-content": {
    caretColor: c.cursor,
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: c.cursor,
  },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    {
      backgroundColor: c.selection,
    },
  ".cm-selectionLayer .cm-selectionBackground": {
    backgroundColor: c.inactiveSelection,
  },
  ".cm-gutters": {
    backgroundColor: c.background,
    color: c.lineNumber,
    border: "none",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: c.activeLineNumber,
  },
  ".cm-activeLine": {
    backgroundColor: "transparent",
  },
  "&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket": {
    backgroundColor: c.matchingBracket,
    outline: `1px solid ${c.matchingBracketBorder}`,
  },
  ".cm-indentGuide": {
    borderLeft: `1px solid ${c.indentGuide}`,
  },
});

export const vscodeTheme = (): Extension => [
  vscodeEditorTheme,
  syntaxHighlighting(vscodeHighlightStyle),
];
