import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import path from 'path'

export default defineConfig({
  plugins: [react(), wasm(), topLevelAwait()],
  assetsInclude: ['**/*.wasm'],
  resolve: {
    alias: {
      '@core': path.resolve(__dirname, 'src/core'),
      '@features': path.resolve(__dirname, 'src/features'),
      '@shared': path.resolve(__dirname, 'src/shared'),
      '@pages': path.resolve(__dirname, 'src/pages'),
    },
  },
  server: {
    port: 1420,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3737',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://127.0.0.1:3737',
        ws: true,
      },
    },
  },
  build: {
    target: 'es2020',
    // Smaller chunk warning threshold (Vite default is 500KB; this catches bloated chunks early)
    chunkSizeWarningLimit: 400,
    minify: 'esbuild',
    cssMinify: true,
    rollupOptions: {
      output: {
        // Manual chunks split heavy dependencies into their own bundles so they
        // can be cached independently and don't bloat the main entry.
        // The chunk NAMES (keys) become the file prefix.
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined

          // React core — used everywhere, split for long-term caching
          if (
            id.includes('node_modules/react/') ||
            id.includes('node_modules/react-dom/') ||
            id.includes('node_modules/scheduler/')
          ) {
            return 'react-vendor'
          }

          // TanStack router — large, used in shell + every page
          if (id.includes('node_modules/@tanstack/')) {
            return 'router-vendor'
          }

          // Tiptap editor stack — heavy, only needed inside PageView
          if (
            id.includes('node_modules/@tiptap/') ||
            id.includes('node_modules/prosemirror-') ||
            id.includes('node_modules/orderedmap')
          ) {
            return 'tiptap-vendor'
          }

          // dnd-kit — only used by PageView drag and drop
          if (id.includes('node_modules/@dnd-kit/')) {
            return 'dnd-vendor'
          }

          // react-virtuoso — virtualized list in PageView
          if (id.includes('node_modules/react-virtuoso')) {
            return 'virtuoso-vendor'
          }

          // Icons — large when tree-shaken poorly
          if (id.includes('node_modules/lucide-react')) {
            return 'icons-vendor'
          }

          // Hot toast — small but used in shell
          if (id.includes('node_modules/react-hot-toast')) {
            return 'toast-vendor'
          }

          // Anything else from node_modules goes into a misc vendor chunk
          return 'vendor-misc'
        },
      },
    },
  },
  optimizeDeps: {
    exclude: ['quilt-core'], // WASM module loaded at runtime
  },
})
