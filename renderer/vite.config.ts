import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        format: "iife",
        name: "TamtriTranscript",
        inlineDynamicImports: true,
        entryFileNames: "assets/transcript.js",
        assetFileNames: "assets/transcript.[ext]",
      },
    },
  },
});
