// https://github.com/garrynewman/bootil/blob/beb4cec8ad29533965491b767b177dc549e62d23/src/3rdParty/globber.cpp
// https://github.com/Facepunch/gmad/blob/master/include/AddonWhiteList.h

pub const DEFAULT_IGNORE: &'static [&'static str] = &[
	".git/*",
	"*.psd",
	"*.pdn",
	"*.xcf",
	"*.svn",
	"*.ini",
	".DS_Store",
	".gitignore",
	".vscode/*",
	".github/*",
	".vs/*",
	".editorconfig",
	"README.md",
	"README.txt",
	"readme.txt",
	"addon.json",
	"addon.txt",
	"addon.jpg",
];

const ADDON_WHITELIST: &'static [&'static str] = &[
	"lua/*.lua",
	"scenes/*.vcd",
	"particles/*.pcf",
	"resource/fonts/*.ttf",
	"scripts/vehicles/*.txt",
	"resource/localization/*/*.properties",
	"maps/*.bsp",
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
];

const WILD_BYTE: u8 = '*' as u8;
const QUESTION_BYTE: u8 = '?' as u8;

pub fn globber(wild: &str, str: &str) -> bool {
	let mut cp = 0;
	let mut mp = 0;
	let mut s = 0;
	let mut w = 0;
	let wild_len = wild.len();
	let str_len = str.len();
	let wild_bytes = wild.as_bytes();
	let str_bytes = str.as_bytes();

	while s != str_len && w != wild_len && wild_bytes[w] != WILD_BYTE {
		if wild_bytes[w] != str_bytes[s] && wild_bytes[w] != QUESTION_BYTE {
			return false;
		}

		s += 1;
		w += 1;
	}

	while s != str_len {
		if wild_bytes.get(w).map(|c| *c == WILD_BYTE).unwrap_or(false) {
			w += 1;
			if w == wild_len {
				return true;
			}

			mp = w;
			cp = s + 1;
		} else if wild_bytes[w] == str_bytes[s] || wild_bytes[w] == QUESTION_BYTE {
			s += 1;
			w += 1;
		} else {
			w = mp;
			s = cp;
			cp += 1;
		}
	}

	while w != wild_len && wild_bytes[w] == WILD_BYTE {
		w += 1;
	}

	wild_len == w
}

pub fn check(str: &str) -> bool {
	for glob in ADDON_WHITELIST {
		if globber(glob, str) {
			return true;
		}
	}

	false
}

pub fn filter_default_ignored(str: &str) -> bool {
	for glob in DEFAULT_IGNORE {
		if globber(glob, str) {
			return false;
		}
	}

	true
}

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
		"GAMEMODES/MY_BASE_DEFENCE/BACKGROUNDS/1.JPG",
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
		assert!(check(&*good));
	}

	for good in ADDON_WHITELIST {
		assert!(check(&good.replace('*', "test")));
	}

	for good in ADDON_WHITELIST {
		assert!(check(&good.replace('*', "a")));
	}

	for bad in bad {
		assert!(!check(&*bad));
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
		assert!(!filter_default_ignored(&*ignored));
	}

	let default_ignore: Vec<String> = DEFAULT_IGNORE.iter().cloned().map(|x| x.to_string()).collect();
	for ignored in ignored {
		assert!(is_ignored(&*ignored, &default_ignore));
	}

	assert!(is_ignored(&"lol.txt".to_string(), &["lol.txt".to_string()]));
	assert!(!is_ignored(&"lol.txt".to_string(), &[]));
}
