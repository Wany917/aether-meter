import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// One page per window: the overlays stay tiny, the hub carries the rest.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    rolldownOptions: {
      input: {
        meter: "meter.html",
        tracker: "tracker.html",
        routine: "routine.html",
        hub: "hub.html",
      },
    },
  },
});
