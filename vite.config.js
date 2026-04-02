import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  root: "src",
  base: "./",
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
    rollupOptions: {
      input: {
        main: "src/index.html",
        routerSettings: "src/router-settings.html",
      },
    },
  },
});
