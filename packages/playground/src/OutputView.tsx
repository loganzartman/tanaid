import { EditorState } from "@codemirror/state";
import { drawSelection, EditorView } from "@codemirror/view";
import { createEffect, onSettled } from "solid-js";
import { emacsTheme } from "./theme.ts";

/**
 * Read-only view of text that grows over time, kept scrolled to the end.
 * When `text` extends the previous value, only the new part is inserted.
 */
export function OutputView(props: { text: string }) {
  const view = new EditorView({
    state: EditorState.create({
      doc: "",
      extensions: [EditorState.readOnly.of(true), drawSelection(), emacsTheme()],
    }),
  });

  createEffect(
    () => props.text,
    (text, prev) => {
      const from = prev !== undefined && text.startsWith(prev) ? prev.length : 0;
      view.dispatch({ changes: { from, to: view.state.doc.length, insert: text.slice(from) } });
      view.requestMeasure({
        key: "scroll-to-end",
        read: () => view.scrollDOM.scrollHeight,
        write: (scrollHeight) => {
          view.scrollDOM.scrollTop = scrollHeight;
        },
      });
    },
  );

  onSettled(() => () => view.destroy());

  return view.dom;
}
