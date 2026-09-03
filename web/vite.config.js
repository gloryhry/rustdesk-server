import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 5178,
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY_TARGET || 'http://127.0.0.1:21114',
        changeOrigin: true
      }
    }
  }
});
