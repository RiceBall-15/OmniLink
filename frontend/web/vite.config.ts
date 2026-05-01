import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    host: '0.0.0.0',
    strictPort: true,
    // 限制并发编译，减少内存占用
    watch: {
      usePolling: false,
    },
    // 代理配置
    proxy: {
      '/api': {
        target: 'http://localhost:8002',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:8001',
        ws: true,
      },
    },
    // HMR优化
    hmr: {
      overlay: false, // 禁用错误覆盖层，减少内存
    },
  },
  // 构建优化
  build: {
    // 减少chunk大小
    chunkSizeWarningLimit: 500,
    // 最小化配置
    minify: 'esbuild',
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
        },
      },
    },
  },
  // 依赖优化
  optimizeDeps: {
    include: ['react', 'react-dom'],
  },
})
