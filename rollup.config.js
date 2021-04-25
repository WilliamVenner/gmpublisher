import svelte from 'rollup-plugin-svelte';
import commonjs from '@rollup/plugin-commonjs';
import resolve from '@rollup/plugin-node-resolve';
import livereload from 'rollup-plugin-livereload';
import replace from '@rollup/plugin-replace';
import dynamicImportVars from '@rollup/plugin-dynamic-import-vars';
import json from '@rollup/plugin-json';
import { terser } from 'rollup-plugin-terser';
import css from 'rollup-plugin-css-only';
import svelteSVG from "rollup-plugin-svelte-svg";
import fs from 'fs';

const appLanguages = {};
const languageFiles = fs.readdirSync('./i18n');
let i = -1;
while (++i < languageFiles.length) {
	const file = languageFiles[i];
	if (file === 'en.json') continue;
	const fileName = file.substr(0, file.length - 5);
	let languageName;
	try { languageName = JSON.parse(fs.readFileSync('./i18n/' + file, { encoding: 'utf-8' }))?.LANGUAGE_NAME ?? file; } catch(e) {}
	appLanguages[fileName] = languageName ?? fileName;
}

const production = !process.env.ROLLUP_WATCH;

function serve() {
	let server;

	function toExit() {
		if (server) server.kill(0);
	}

	return {
		writeBundle() {
			if (server) return;
			server = require('child_process').spawn('npm', ['run', 'start', '--', '--dev'], {
				stdio: ['ignore', 'inherit', 'inherit'],
				shell: true
			});

			process.on('SIGTERM', toExit);
			process.on('exit', toExit);
		}
	};
}

export default {
	input: 'app/main.js',
	output: {
		sourcemap: false,
		format: 'iife',
		name: 'app',
		file: 'public/build/bundle.js',
		inlineDynamicImports: true,

		intro: `const APP_LANGUAGES = ${JSON.stringify(appLanguages)};`
	},
	plugins: [
		replace({
			'process.env.NODE_ENV': JSON.stringify(
				production ? 'production' : 'development'
			),
		}),
		svelte({
			compilerOptions: {
				// enable run-time checks when not in production
				dev: !production
			}
		}),
		// we'll extract any component CSS out into
		// a separate file - better for performance
		css({ output: 'bundle.css' }),

		// If you have external dependencies installed from
		// npm, you'll most likely need these plugins. In
		// some cases you'll need additional configuration -
		// consult the documentation for details:
		// https://github.com/rollup/plugins/tree/master/packages/commonjs
		resolve({
			browser: true,
			dedupe: ['svelte']
		}),
		commonjs(),

		json(),

		svelteSVG(),

		dynamicImportVars({
			//include: languageFiles.map(file => './i18n/' + file)
		}),

		// In dev mode, call `npm run start` once
		// the bundle has been generated
		!production && serve(),

		// Watch the `public` directory and refresh the
		// browser on changes when not in production
		!production && livereload('public'),

		// If we're building for production (npm run build
		// instead of npm run dev), minify
		production && terser({ format: { comments: false } })
	],
	watch: {
		clearScreen: false
	}
};
