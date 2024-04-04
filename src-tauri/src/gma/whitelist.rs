// https://github.com/garrynewman/bootil/blob/beb4cec8ad29533965491b767b177dc549e62d23/src/3rdParty/globber.cpp
// https://github.com/Facepunch/gmad/blob/master/include/AddonWhiteList.h

use std::time::Duration;

macro_rules! globbers {
	($($glob:literal),*) => {
		&[
			$(concat!($glob, '\0')),*
		]
	};
}

pub const DEFAULT_IGNORE: &[&str] = globbers!(
	".git/*",
	"*.psd",
	"*.pdn",
	"*.xcf",
	"*.svn",
	"*.ini",
	"*.rtf",
	"*.pdf",
	".DS_Store",
	".gitignore",
	".gitmodules",
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
	"desktop.ini"
);

const ADDON_WHITELIST_OFFLINE: &[&str] = globbers!(
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
	"models/*.vtx",
	"models/*.phy",
	"models/*.ani",
	"models/*.vvd",
	"gamemodes/*/*.txt",
	"gamemodes/*/*.fgd",
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
	"gamemodes/*/content/models/*.vtx",
	"gamemodes/*/content/models/*.phy",
	"gamemodes/*/content/models/*.ani",
	"gamemodes/*/content/models/*.vvd",
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
	"data_static/*.dem",
	"data_static/*.vcd",
	"data_static/*.vtf",
	"data_static/*.vmt",
	"data_static/*.png",
	"data_static/*.jpg",
	"data_static/*.jpeg",
	"data_static/*.mp3",
	"data_static/*.wav",
	"data_static/*.ogg"
);

lazy_static! {
	pub static ref ADDON_WHITELIST: &'static [&'static str] = download_addon_whitelist();
}

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
				.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to parse addon whitelist"))?;

			for line in captures.as_str().lines() {
				let line = line.trim();
				if line.is_empty() {
					continue;
				} else if line == "NULL" {
					break;
				} else if line.get(0..1) == Some("\"") && line.get(line.len() - 2..line.len()) == Some("\",") {
					wildcard.push(&*format!("{}\0", &line[1..line.len() - 2]).leak());
				}
			}

			if wildcard.is_empty() {
				return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to parse addon whitelist (empty)"));
			}

			if !wildcard.contains(&"lua/*.lua\0") {
				// This should definitely be in there, so if it isn't, something has gone wrong. Probably.
				return Err(std::io::Error::new(
					std::io::ErrorKind::Other,
					"Failed to parse addon whitelist (missing lua/*.lua)",
				));
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

pub fn globber(_wild: &str, _str: &str) -> bool {
	debug_assert!(_wild.ends_with('\0'), "wild must be null terminated");
	debug_assert!(!_wild.ends_with("\0\0"), "wild is double null terminated");
	debug_assert!(_str.ends_with('\0'), "str must be null terminated");
	debug_assert!(!_str.ends_with("\0\0"), "str is double null terminated");
	unsafe {
		let mut cp: *const u8 = std::ptr::null::<u8>();
		let mut mp: *const u8 = std::ptr::null::<u8>();

		let mut wild = _wild.as_ptr();
		let mut str = _str.as_ptr();

		while *str != 0 && *wild != WILD_BYTE {
			if *wild != *str && *wild != QUESTION_BYTE {
				return false;
			}
			wild = wild.add(1);
			str = str.add(1);
		}

		while *str != 0 {
			if *wild == WILD_BYTE {
				wild = wild.add(1);
				if *wild == 0 {
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

		while *wild == WILD_BYTE {
			wild = wild.add(1);
		}
		*wild == 0
	}
}

pub fn check(str: &str) -> bool {
	let mut str = str.to_string();
	str.push('\0');

	for glob in ADDON_WHITELIST.iter() {
		if globber(glob, &str) {
			return true;
		}
	}

	false
}

pub fn filter_default_ignored(str: &str) -> bool {
	let mut str = str.to_string();
	str.push('\0');

	for glob in DEFAULT_IGNORE {
		if globber(glob, &str) {
			return false;
		}
	}

	true
}

pub fn is_ignored(str: &str, ignore: &[String]) -> bool {
	if ignore.is_empty() {
		return false;
	}

	let mut str = str.to_string();
	str.push('\0');

	for glob in ignore {
		if globber(glob, &str) {
			return true;
		}
	}

	false
}

#[test]
pub fn test_whitelist() {
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
pub fn test_ignore() {
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
