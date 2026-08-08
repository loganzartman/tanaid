import { defineConfig } from 'tsdown'
import { wasm } from 'rolldown-plugin-wasm';

export default defineConfig({
  entry: ['./src/index.ts'],
  plugins: [wasm()],
});
