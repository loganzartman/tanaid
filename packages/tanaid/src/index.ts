import {
  Interpreter,
  InterpreterOptions,
} from "../../../crates/tanaid-wasm/pkg/bundler/tanaid_wasm";
export * from "../../../crates/tanaid-wasm/pkg/bundler/tanaid_wasm";

export type CreateInterpreterOptions = Omit<
  InterpreterOptions,
  "handleStdout" | "handleEventLoopStatus" | "setTimeout" | "clearTimeout"
> & {
  handleStdout?: InterpreterOptions["handleStdout"];
  handleEventLoopStatus?: InterpreterOptions["handleEventLoopStatus"];
  setTimeout?: InterpreterOptions["setTimeout"];
  clearTimeout?: InterpreterOptions["clearTimeout"];
};

/** Interpreter.create with default options for convenience */
export function createInterpreter(options: CreateInterpreterOptions): Interpreter {
  return Interpreter.create({
    handleStdout: (stdout: string) => {
      console.log(stdout);
    },

    handleEventLoopStatus: (_nPending: number) => {},

    setTimeout: (callback, delayMs) => {
      return globalThis.setTimeout(callback, delayMs);
    },

    clearTimeout: (timeoutId) => {
      globalThis.clearTimeout(timeoutId as number);
    },

    ...options,
  } satisfies InterpreterOptions);
}
