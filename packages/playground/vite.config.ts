import { defineConfig } from "vite";

const target = ["chrome132", "edge132", "firefox134", "safari18.2"];

export default defineConfig({
  build: { target },
  worker: {
    format: "es",
  },
});
