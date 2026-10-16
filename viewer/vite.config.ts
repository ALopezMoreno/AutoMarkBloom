import { defineConfig } from "vite";
export default defineConfig({
  base: "./",
  build: { target: "es2022" },
  worker: { format: "es" },
  // The optional wasm build lives in ../wasm/pkg.
  server: { fs: { allow: [".."] } },
});
