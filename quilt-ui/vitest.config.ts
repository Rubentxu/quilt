import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

// Vitest config for component + hook tests.
//
// The existing vite.config.ts owns the dev server, proxy, WASM plugin
// chain, and chunk-splitting. We don't extend it here because:
//   - vitest doesn't need the WASM plugin (component tests mock it)
//   - the dev-server proxy is irrelevant for jsdom
//   - the manualChunks function would still run for vitest builds and
//     just produce noise.
//
// The aliases intentionally mirror the ones in vite.config.ts and
// tsconfig.json (paths section). If you add a new alias there, add
// it here too.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    // The bundle test in src/__tests__/bundle.test.ts needs `dist/`.
    // In CI we run it after `npm run build`; locally you can skip it
    // with `npx vitest run --exclude src/__tests__/bundle.test.ts`.
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
    },
  },
  resolve: {
    alias: {
      '@core': path.resolve(__dirname, 'src/core'),
      '@features': path.resolve(__dirname, 'src/features'),
      '@pages': path.resolve(__dirname, 'src/pages'),
      '@shared': path.resolve(__dirname, 'src/shared'),
    },
  },
})
