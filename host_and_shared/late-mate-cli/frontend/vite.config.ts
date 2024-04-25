import { defineConfig } from "vite";

// noinspection JSUnusedGlobalSymbols
export default defineConfig({
  server: {
    proxy: {
      "/ws": {
        target: "ws://127.0.0.1:9118",
        ws: true,
        changeOrigin: true
      }
    }
  }
});
