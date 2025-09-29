import { defineConfig } from 'vite'

/**
 * Vite configuration for single-port architecture
 * Per spec-kit/004-frontend-spec.md section 2.2
 *
 * CRITICAL CONSTRAINTS:
 * - base: './' ensures all paths are relative (no hardcoded URLs)
 * - Proxy is ONLY for development mode
 * - Production build is served statically from Rust backend on port 8080
 * - WebSocket URLs constructed dynamically from window.location
 */
export default defineConfig({
  base: './',  // RELATIVE PATHS ONLY - required for single-port architecture

  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          'xterm': ['xterm', 'xterm-addon-fit', 'xterm-addon-web-links']
        }
      }
    }
  },

  // Development server configuration (not used in production)
  server: {
    port: 5173,
    proxy: {
      // Proxy WebSocket connections during development
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true
      },
      // Proxy API requests during development
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true
      }
    }
  },

  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './tests/setup.ts'
  }
})