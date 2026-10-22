import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Fixed port so the Tauri dev shell can load it.
export default defineConfig({
  plugins: [react()],
  base: './',
  clearScreen: false,
  server: { port: 5173, strictPort: true },
});
