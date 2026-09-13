import { globSync, readFileSync } from "node:fs";

export function loadExamples(): Record<string, string> {
  const examples: Record<string, string> = {};
  console.log(process.cwd());
  const paths = globSync("../../sample/*.tcl");
  console.log(paths);
  for (const path of paths) {
    const name = /.*\/([^.]+)(\..*)?$/.exec(path)?.[1];
    if (typeof name !== "string") {
      continue;
    }
    const content = readFileSync(path, "utf-8");
    examples[name] = content;
  }
  return examples;
}
