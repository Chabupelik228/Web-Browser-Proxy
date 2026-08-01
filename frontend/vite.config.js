// frontend/vite.config.js
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    rollupOptions: {
      // Говорим сборщику не пытаться искать эти файлы локально на этапе сборки
      external: [
        '/config.js',
        '/scram/scramjet.bundle.js',
        '/scram/scramjet.all.js',
        '/scram/scramjet.sync.js',
        '/scram/scramjet.wasm.wasm',
        '/baremux/index.js',
        '/baremux/worker.js',
        '/libcurl/index.mjs'
      ],
    },
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/scram': 'http://127.0.0.1:8080',
      '/libcurl': 'http://127.0.0.1:8080',
      '/baremux': 'http://127.0.0.1:8080',
      '/wisp/': {
        target: 'ws://127.0.0.1:8080',
        ws: true
      }
    }
  }
})