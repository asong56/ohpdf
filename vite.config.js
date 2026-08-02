import { defineConfig } from 'vite';

// Tauri 官方模板的标准 vite 配置：固定端口给 tauri.conf.json 里的
// devUrl 使用，并让 Vite 忽略 src-tauri（避免 Rust 编译产物触发前端热更新）。
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
