import { defineConfig } from 'tsdown'
import { wasm } from 'rolldown-plugin-wasm';

export default defineConfig({
  dts: true,
  entry: ['./src/index.ts'],
  plugins: [wasm()],
});
