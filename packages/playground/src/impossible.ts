export function impossible(x: never): never {
  throw new Error(`impossible case: ${x}`);
}
