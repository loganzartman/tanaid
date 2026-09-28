import { For } from "solid-js";

/** Menu of example programs. It returns to its placeholder after each pick. */
export function ExampleSelect(props: {
  examples: Record<string, string>;
  onSelect: (source: string) => void;
}) {
  return (
    <select
      onChange={(event) => {
        const select = event.currentTarget;
        props.onSelect(select.value);
        select.selectedIndex = 0;
        select.blur();
      }}
    >
      <option value="" selected disabled>
        load an example...
      </option>
      <For each={Object.entries(props.examples)}>
        {([name, source]) => <option value={source}>{name}</option>}
      </For>
    </select>
  );
}
