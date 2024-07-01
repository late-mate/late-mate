import { defineConfig } from 'vite';

// noinspection JSUnusedGlobalSymbols
export default defineConfig({
  // Note that this is important since Zola copies the entire /static
  // into the final result, and I need to separate "built" content, so
  // into the built/ subdir it goes and in-CSS urls must be adjusted
  // accordingly
  base: '/built/',

  build: {
    manifest: true,
    outDir: 'static/built',
    assetsDir: 'assets',
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
