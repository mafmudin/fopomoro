import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Tauri builds Rust artifacts into src-tauri/target; those .dll/.exe files
    // get locked during a build and crash Vite's watcher with EBUSY. Don't watch
    // the Rust side — cargo handles its own rebuilds.
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
})
