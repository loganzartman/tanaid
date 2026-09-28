import { Annotation, EditorState } from "@codemirror/state";
import {
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { tcl } from "@sourcebot/codemirror-lang-tcl";
import { createEffect, onSettled, untrack } from "solid-js";
import { emacsTheme } from "./theme.ts";

/** Marks transactions that sync `props.value` into the editor. */
const syncValue = Annotation.define<boolean>();

/** Tcl source editor. Edits are reported through `onChange`. */
export function CodeEditor(props: { value: string; onChange: (value: string) => void }) {
  const view = new EditorView({
    state: EditorState.create({
      doc: untrack(() => props.value),
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        drawSelection(),
        emacsTheme(),
        tcl(),
        EditorView.updateListener.of((update) => {
          const synced = update.transactions.some((tr) => tr.annotation(syncValue));
          if (update.docChanged && !synced) {
            props.onChange(update.state.doc.toString());
          }
        }),
      ],
    }),
  });

  createEffect(
    () => props.value,
    (value) => {
      if (value === view.state.doc.toString()) return;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
        annotations: syncValue.of(true),
      });
    },
  );

  onSettled(() => () => view.destroy());

  return view.dom;
}
