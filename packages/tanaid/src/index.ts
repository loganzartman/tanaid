import { Tcl, TclOptions } from "../../../crates/tanaid-wasm/pkg/bundler/tanaid_wasm";
export * from "../../../crates/tanaid-wasm/pkg/bundler/tanaid_wasm";

export type CreateTclOptions = Partial<TclOptions>;

/** Tcl.create with default options for convenience */
export function createTcl(options: CreateTclOptions = {}): Tcl {
  return Tcl.create({
    handleStdout: (stdout: string) => {
      console.log(stdout);
    },

    setTimeout: (callback, delayMs) => {
      return globalThis.setTimeout(callback, delayMs);
    },

    clearTimeout: (timeoutId) => {
      globalThis.clearTimeout(timeoutId as number);
    },

    ...options,
  } satisfies TclOptions);
}
