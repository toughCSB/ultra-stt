import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}', 'src/styles/**/*.test.js', 'remote/**/*.test.js'],
    maxWorkers: 4,
    setupFiles: ['src/test-setup.ts'],
  },
})
