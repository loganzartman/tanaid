import { globSync, readFileSync } from "node:fs";
import { basename, dirname } from "node:path";

export function loadExamples(): Record<string, Record<string, string>> {
  const examples: Record<string, Record<string, string>> = {};
  for (const path of globSync("../../sample/*/*.tcl").sort()) {
    const dir = basename(dirname(path));
    const category = dir[0].toUpperCase() + dir.slice(1);
    (examples[category] ??= {})[basename(path, ".tcl")] = readFileSync(path, "utf-8");
  }
  return examples;
}
