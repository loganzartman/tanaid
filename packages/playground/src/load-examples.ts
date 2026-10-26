import { globSync, readFileSync } from "node:fs";

/** Sample programs by category, read from `sample/<category>/<name>.tcl`. */
export function loadExamples(): Record<string, Record<string, string>> {
  const examples: Record<string, Record<string, string>> = {};
  for (const path of globSync("../../sample/*/*.tcl").sort()) {
    const match = /([^/]+)\/([^/.]+)\.tcl$/.exec(path);
    if (!match) {
      continue;
    }
    const [, directory, name] = match;
    const category = directory[0].toUpperCase() + directory.slice(1);
    (examples[category] ??= {})[name] = readFileSync(path, "utf-8");
  }
  return examples;
}
