import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}', 'src/styles/**/*.test.js', 'remote/**/*.test.js'],
    maxWorkers: 4,
    // Node 25+ has its own localStorage, empty without --localstorage-file, which hides jsdom's.
    execArgv:
      Number(process.versions.node.split('.')[0]) >= 25 ? ['--no-experimental-webstorage'] : [],
    setupFiles: ['src/test-setup.ts'],
  },
})
