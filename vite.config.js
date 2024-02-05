import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import replace from '@rollup/plugin-replace';
import json from '@rollup/plugin-json';
import { svelteSVG } from "rollup-plugin-svelte-svg";
import fs from 'fs';

const appLanguages = {};
{
	const languageFiles = fs.readdirSync('./i18n');
	let i = -1;
	while (++i < languageFiles.length) {
		const file = languageFiles[i];
		const fileName = file.substr(0, file.length - 5);
		const languageData = JSON.parse(fs.readFileSync('./i18n/' + file, { encoding: 'utf-8' }));
		appLanguages[fileName] = languageData;
	}
}

const production = process.env.TAURI_DEBUG != "true";

console.log('production', production);

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [svelte()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  root: "app",
  publicDir: "../public",
  build: {
    outDir: "./public/build",
    rollupOptions: {
      input: {
        app: './app/index.html',
      },
      output: {
        intro: `window.APP_LANGUAGES = ${JSON.stringify(appLanguages)};`
      },
      plugins: [
        replace({
          'process.env.NODE_ENV': JSON.stringify(
            production ? 'production' : 'development'
          ),
        }),
        json(),
        svelteSVG(),
      ]
    },
  },
}));
