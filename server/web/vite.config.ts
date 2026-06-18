import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Dev server proxies the API and static assets to the axum server on :3000, so
// the browser only ever talks to Vite (no CORS, single origin).
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:3000",
      "/assets": "http://127.0.0.1:3000",
    },
  },
});
