// https://github.com/garrynewman/bootil/blob/beb4cec8ad29533965491b767b177dc549e62d23/src/3rdParty/globber.cpp
// https://github.com/Facepunch/gmad/blob/master/include/AddonWhiteList.h

use std::{sync::LazyLock, time::Duration};

const ADDON_WHITELIST_OFFLINE: &[&str] = &[
	"lua/*.lua",
	"scenes/*.vcd",
	"particles/*.pcf",
	"resource/fonts/*.ttf",
	"scripts/vehicles/*.txt",
	"resource/localization/*/*.properties",
	"maps/*.bsp",
	"maps/*.lmp",
	"maps/*.nav",
	"maps/*.ain",
	"maps/thumb/*.png",
	"sound/*.wav",
	"sound/*.mp3",
	"sound/*.ogg",
	"materials/*.vmt",
	"materials/*.vtf",
	"materials/*.png",
	"materials/*.jpg",
	"materials/*.jpeg",
	"materials/colorcorrection/*.raw",
	"models/*.mdl",
	"models/*.phy",
	"models/*.ani",
	"models/*.vvd",
	"models/*.vtx",
	"!models/*.sw.vtx",
	"!models/*.360.vtx",
	"!models/*.xbox.vtx",
	"gamemodes/*/*.txt",
	"!gamemodes/*/*/*.txt",
	"gamemodes/*/*.fgd",
	"!gamemodes/*/*/*.fgd",
	"gamemodes/*/logo.png",
	"gamemodes/*/icon24.png",
	"gamemodes/*/gamemode/*.lua",
	"gamemodes/*/entities/effects/*.lua",
	"gamemodes/*/entities/weapons/*.lua",
	"gamemodes/*/entities/entities/*.lua",
	"gamemodes/*/backgrounds/*.png",
	"gamemodes/*/backgrounds/*.jpg",
	"gamemodes/*/backgrounds/*.jpeg",
	"gamemodes/*/content/models/*.mdl",
	"gamemodes/*/content/models/*.phy",
	"gamemodes/*/content/models/*.ani",
	"gamemodes/*/content/models/*.vvd",
	"gamemodes/*/content/models/*.vtx",
	"!gamemodes/*/content/models/*.sw.vtx",
	"!gamemodes/*/content/models/*.360.vtx",
	"!gamemodes/*/content/models/*.xbox.vtx",
	"gamemodes/*/content/materials/*.vmt",
	"gamemodes/*/content/materials/*.vtf",
	"gamemodes/*/content/materials/*.png",
	"gamemodes/*/content/materials/*.jpg",
	"gamemodes/*/content/materials/*.jpeg",
	"gamemodes/*/content/materials/colorcorrection/*.raw",
	"gamemodes/*/content/scenes/*.vcd",
	"gamemodes/*/content/particles/*.pcf",
	"gamemodes/*/content/resource/fonts/*.ttf",
	"gamemodes/*/content/scripts/vehicles/*.txt",
	"gamemodes/*/content/resource/localization/*/*.properties",
	"gamemodes/*/content/maps/*.bsp",
	"gamemodes/*/content/maps/*.nav",
	"gamemodes/*/content/maps/*.ain",
	"gamemodes/*/content/maps/thumb/*.png",
	"gamemodes/*/content/sound/*.wav",
	"gamemodes/*/content/sound/*.mp3",
	"gamemodes/*/content/sound/*.ogg",
	"data_static/*.txt",
	"data_static/*.dat",
	"data_static/*.json",
	"data_static/*.xml",
	"data_static/*.csv",
	"shaders/*.vcs",
];

pub const DEFAULT_IGNORE: &[&str] = &[
	".git/*",
	"*.psd",
	"*.pdn",
	"*.xcf",
	"*.kra",
	"*.svn",
	"*.ini",
	"*.rtf",
	"*.pdf",
	"*.log",
	"*.prt",
	"*.vmf",
	"*.vmx",
	".DS_Store",
	".gitignore",
	".gitmodules",
	".gitattributes",
	".vscode/*",
	".github/*",
	".vs/*",
	".editorconfig",
	"LICENSE",
	"LICENSE.*",
	"license",
	"license.*",
	"README",
	"README.*",
	"readme",
	"readme.*",
	"addon.json",
	"addon.txt",
	"addon.jpg",
	"thumbs.db",
	"desktop.ini",
	"models/*.sw.vtx",
	"models/*.360.vtx",
	"models/*.xbox.vtx",
	"gamemodes/*/content/models/*.sw.vtx",
	"gamemodes/*/content/models/*.360.vtx",
	"gamemodes/*/content/models/*.xbox.vtx",
];

pub static ADDON_WHITELIST: LazyLock<&'static [&'static str]> = LazyLock::new(download_addon_whitelist);

fn download_addon_whitelist() -> &'static [&'static str] {
	if std::env::var_os("ADDON_WHITELIST_OFFLINE").is_some() {
		return ADDON_WHITELIST_OFFLINE;
	}

	ureq::get("https://raw.githubusercontent.com/Facepunch/gmad/master/include/AddonWhiteList.h")
		.timeout(Duration::from_secs(2))
		.call()
		.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
		.and_then(|response| response.into_string())
		.and_then(|response| {
			let mut wildcard = Vec::new();

			let captures = regex::Regex::new(r#"static +const +char\* +Wildcard\s*\[\s*\]\s*=\s*\{\s*([\s\S]*?)\s*NULL,?\s*};"#)
				.unwrap()
				.captures(response.leak())
				.and_then(|captures| captures.get(1))
				.ok_or_else(|| std::io::Error::other("Failed to parse addon whitelist"))?;

			let line_regex = regex::Regex::new(r#""(.+?)","#).unwrap();

			for line in captures.as_str().lines() {
				let line = line.trim();
				if line.is_empty() {
					continue;
				} else if line == "NULL" {
					break;
				} else if let Some(capture) = line_regex.captures(line) {
					let glob = capture.get(1).unwrap().as_str();
					wildcard.push(&*glob.to_string().leak());
				}
			}

			if wildcard.is_empty() {
				return Err(std::io::Error::other("Failed to parse addon whitelist (empty)"));
			}

			if !wildcard.contains(&"lua/*.lua") {
				// This should definitely be in there, so if it isn't, something has gone wrong. Probably.
				return Err(std::io::Error::other("Failed to parse addon whitelist (missing lua/*.lua)"));
			}

			println!("Downloaded up to date addon whitelist: {wildcard:#?}");

			Ok(&*wildcard.leak())
		})
		.map_err(|err| {
			eprintln!("Failed to download addon whitelist: {:#?}", err);
			err
		})
		.unwrap_or(ADDON_WHITELIST_OFFLINE)
}

const WILD_BYTE: u8 = b'*';
const QUESTION_BYTE: u8 = b'?';
const EXCLAMATION_BYTE: u8 = b'!';

fn globber(wild: &str, str: &str) -> bool {
	unsafe {
		let mut cp: *const u8 = core::ptr::null();
		let mut mp: *const u8 = core::ptr::null();

		let (mut wild, wild_max) = (wild.as_ptr(), wild.as_ptr().add(wild.len()));
		let (mut str, str_max) = (str.as_ptr(), str.as_ptr().add(str.len()));

		while wild < wild_max && str < str_max && *wild != WILD_BYTE {
			if *wild != *str && *wild != QUESTION_BYTE {
				return false;
			}
			wild = wild.add(1);
			str = str.add(1);
		}

		while str < str_max {
			if *wild == WILD_BYTE {
				wild = wild.add(1);
				if wild >= wild_max {
					return true;
				}
				mp = wild;
				cp = str.add(1);
			} else if *wild == *str || *wild == QUESTION_BYTE {
				wild = wild.add(1);
				str = str.add(1);
			} else {
				wild = mp;
				str = cp;
				cp = cp.add(1);
			}
		}

		while wild < wild_max && *wild == WILD_BYTE {
			wild = wild.add(1);
		}

		wild >= wild_max
	}
}

/// Check if a path is allowed in a GMA file
pub fn check(str: &str) -> bool {
	let mut valid = false;

	for glob in ADDON_WHITELIST.iter() {
		if glob.as_bytes().first() == Some(&EXCLAMATION_BYTE) {
			if globber(&glob[1..], str) {
				valid = false;
			}
		} else if !valid && globber(glob, str) {
			valid = true;
		}
	}

	valid
}

pub fn filter_default_ignored(str: &str) -> bool {
	for glob in DEFAULT_IGNORE {
		if globber(glob, str) {
			return false;
		}
	}

	true
}

/// Check if a path is ignored by a list of custom globs
pub fn is_ignored(str: &str, ignore: &[String]) -> bool {
	if ignore.is_empty() {
		return false;
	}

	for glob in ignore {
		if globber(glob, str) {
			return true;
		}
	}

	false
}

#[test]
fn test_whitelist() {
	let good: &'static [&'static str] = &[
		"lua/test.lua",
		"lua/lol/test.lua",
		"lua/lua/testing.lua",
		"gamemodes/test/something.txt",
		"gamemodes/test/content/sound/lol.wav",
		"materials/lol.jpeg",
		"gamemodes/the_gamemode_name/backgrounds/file_name.jpg",
		"gamemodes/my_base_defence/backgrounds/1.jpg",
	];

	let bad: &'static [&'static str] = &[
		"test.lua",
		"lua/test.exe",
		"lua/lol/test.exe",
		"gamemodes/test",
		"gamemodes/test/something",
		"gamemodes/test/something/something.exe",
		"gamemodes/test/content/sound/lol.vvv",
		"materials/lol.vvv",
	];

	for good in good {
		assert!(check(good), "{}", good);
	}

	for good in ADDON_WHITELIST.iter() {
		assert!(check(good.replace('*', "test").strip_suffix('\0').unwrap()));
	}

	for good in ADDON_WHITELIST.iter() {
		assert!(check(good.replace('*', "a").strip_suffix('\0').unwrap()));
	}

	for bad in bad {
		assert!(!check(bad));
	}
}

#[test]
fn test_ignore() {
	let ignored: &'static [&'static str] = &[
		".git/index",
		".git/info/exclude",
		".git/logs/head",
		".git/logs/refs/heads/4.0.0",
		".git/logs/refs/heads/master",
		".git/logs/refs/remotes/origin/4.0.0",
		".git/logs/refs/remotes/origin/cracker",
		".git/logs/refs/remotes/origin/cracker-no-minigames",
		".git/logs/refs/remotes/origin/master",
		".git/objects/00/007c75922055623f4177467fd50a7d573c2c86",
		"blah.psd",
		"some/location/blah.psd",
		"some/blah/blah.pdn",
		"hi.xcf",
		"addon.jpg",
		"addon.json",
	];

	for ignored in ignored {
		assert!(!filter_default_ignored(ignored));
	}

	let default_ignore: Vec<String> = DEFAULT_IGNORE.iter().cloned().map(|x| x.to_string()).collect();
	for ignored in ignored {
		assert!(is_ignored(ignored, &default_ignore));
	}

	assert!(is_ignored("lol.txt", &["lol.txt\0".to_string()]));
	assert!(is_ignored("lua/hello.lua", &["lua/*.lua\0".to_string()]));
	assert!(is_ignored("lua/hello.lua", &["lua/*\0".to_string()]));
	assert!(!is_ignored("lol.txt", &[]));
}

#[test]
fn test_exclusions() {
	assert!(check("models/player.vtx"));
	assert!(check("models/weapons/gun.vtx"));

	assert!(!check("models/player.sw.vtx"));
	assert!(!check("models/player.360.vtx"));
	assert!(!check("models/player.xbox.vtx"));
	assert!(!check("models/weapons/gun.sw.vtx"));

	assert!(check("gamemodes/test/content/models/player.vtx"));
	assert!(!check("gamemodes/test/content/models/player.sw.vtx"));
	assert!(!check("gamemodes/test/content/models/player.360.vtx"));
	assert!(!check("gamemodes/test/content/models/player.xbox.vtx"));

	assert!(check("gamemodes/sandbox/info.txt"));
	assert!(check("gamemodes/sandbox/sandbox.fgd"));
	assert!(!check("gamemodes/sandbox/nested/info.txt"));
	assert!(!check("gamemodes/sandbox/entities/weapons/info.txt"));
}
