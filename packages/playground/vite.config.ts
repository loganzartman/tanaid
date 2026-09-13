import { defineConfig } from "vite";
import macrosPlugin from "unplugin-macros/vite";

const target = ["chrome132", "edge132", "firefox134", "safari18.2"];

export default defineConfig({
  build: { target },
  worker: {
    format: "es",
  },
  plugins: [macrosPlugin()],
});
