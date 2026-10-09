export function raceAbort<T>(promise: Promise<T>, signal: AbortSignal): Promise<T> {
  return new Promise((res, rej) => {
    promise.then(res).catch(rej);
    signal.addEventListener("abort", () => {
      rej(signal.reason);
    });
  });
}
