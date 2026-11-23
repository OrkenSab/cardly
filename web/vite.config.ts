import { svelte } from "@sveltejs/vite-plugin-svelte";
import { existsSync } from "fs";
import { resolve } from "path";
import { defineConfig } from "vite";

const localGenerated = resolve(__dirname, "generated");
const upstreamGenerated = existsSync(localGenerated)
  ? localGenerated
  : resolve(__dirname, "../anki-upstream/out/ts/lib/generated");

export default defineConfig({
  plugins: [svelte()],
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },
  resolve: {
    alias: {
      "@generated": upstreamGenerated,
    },
  },
  server: {
    host: "127.0.0.1",
    port: 5173,
    fs: {
      allow: [__dirname, upstreamGenerated],
    },
    proxy: {
      "/_anki": "http://127.0.0.1:8080",
      "/_app": "http://127.0.0.1:8080",
      "/api": "http://127.0.0.1:8080",
      "/graphs": "http://127.0.0.1:8080",
      "/deck-options": "http://127.0.0.1:8080",
      "/congrats": "http://127.0.0.1:8080",
      "/card-info": "http://127.0.0.1:8080",
      "/change-notetype": "http://127.0.0.1:8080",
      "/import-anki-package": "http://127.0.0.1:8080",
      "/import-csv": "http://127.0.0.1:8080",
      "/import-page": "http://127.0.0.1:8080",
      "/image-occlusion": "http://127.0.0.1:8080",
      "/editor": "http://127.0.0.1:8080",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2020",
  },
});
