import { defineConfig } from 'vite';

// noinspection JSUnusedGlobalSymbols
export default defineConfig({
  build: {
    manifest: true,
    outDir: 'static/built',
    emptyOutDir: true,
    rollupOptions: {
      input: './src/main.ts',
    },
  },

  server: {
    origin: 'http://localhost:5173',
  },

  // /public is served by Zola anyway
  publicDir: false,
});
