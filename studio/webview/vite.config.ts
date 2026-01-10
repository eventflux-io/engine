import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    rollupOptions: {
      input: resolve(__dirname, 'src/main.tsx'),
      output: {
        entryFileNames: 'index.js',
        chunkFileNames: '[name].js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name?.endsWith('.css')) {
            return 'index.css';
          }
          return '[name][extname]';
        },
      },
    },
    // Don't minify for easier debugging during development
    minify: process.env.NODE_ENV === 'production',
    sourcemap: true,
  },
  define: {
    // VS Code webview doesn't have process.env
    'process.env': {},
  },
});
