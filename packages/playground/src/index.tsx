import { render } from "@solidjs/web";
import { App } from "./App.tsx";
import { decodeUrlPayload } from "./url.ts";

const urlPayload = await decodeUrlPayload(window.location.hash.substring(1).trim()).catch((err) => {
  console.error("URL payload error: ", err);
  window.location.hash = "";
  return null;
});

render(() => <App urlPayload={urlPayload} />, document.getElementById("root")!);
