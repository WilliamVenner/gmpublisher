import fs from "fs";

let steamApiDir;
let steamApiFile;
if (process.platform === "win32") {
	steamApiFile = 'steam_api64.dll';
	steamApiDir = 'win64';
} else if (process.platform === "darwin") {
	steamApiFile = 'libsteam_api.dylib';
	steamApiDir = 'osx';
} else {
	steamApiFile = 'libsteam_api.so';
	steamApiDir = 'linux64';
}

fs.mkdirSync('src-tauri/target/debug', { recursive: true });
fs.mkdirSync('src-tauri/target/release', { recursive: true });

const path = `src-tauri/lib/steam_api/redistributable_bin/${steamApiDir}/${steamApiFile}`;
fs.copyFileSync(path, 'src-tauri/target/debug/' + steamApiFile);
fs.copyFileSync(path, 'src-tauri/target/release/' + steamApiFile);
