import { defineConfig } from 'vitest/config';
import path from 'path';

// Per spec-kit/008-testing-spec.md
export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'happy-dom', // Faster than jsdom
    setupFiles: ['./tests/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      exclude: [
        'node_modules/',
        'tests/',
        '**/*.test.ts',
        '**/*.spec.ts',
        'dist/',
        'vite.config.ts',
        'vitest.config.ts',
      ],
      // Per spec-kit/008-testing-spec.md - >75% coverage required for frontend
      thresholds: {
        lines: 75,
        functions: 75,
        branches: 75,
        statements: 75,
      },
    },
    // Fail tests if they take too long
    testTimeout: 10000,
    hookTimeout: 10000,
  },
});