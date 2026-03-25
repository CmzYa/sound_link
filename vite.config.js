import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  root: "src",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    outDir: "../output/dist",
    minify: "terser",
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true,
      },
    },
    rollupOptions: {
      output: {
        manualChunks: {
          vue: ["vue"],
          lucide: ["lucide-vue-next"],
        },
      },
    },
    chunkSizeWarningLimit: 500,
    cssMinify: true,
  },
  optimizeDeps: {
    include: ["vue", "lucide-vue-next"],
  },
});
