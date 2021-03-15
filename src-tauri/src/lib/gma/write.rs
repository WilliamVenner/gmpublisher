use super::{GMAFile, GMA_HEADER, SUPPORTED_GMA_VERSION};
use byteorder::{LittleEndian, WriteBytesExt};
use std::time::SystemTime;
use std::io;
use std::io::{Write};
use std::ffi::CString;

trait WriteCStrExt: Write {
	fn write_cstr(&mut self, s: &str) -> Result<(), io::Error> {
		let cstr = CString::new(s)?;
		let bytes = cstr.as_bytes_with_nul();
		self.write_all(bytes)
	}
}
impl<W: io::Write + ?Sized> WriteCStrExt for W {}

pub fn write_gma<W: Write>(file: &GMAFile, w: &mut W) -> Result<(), io::Error> {
	w.write(&GMA_HEADER[..])?;
	w.write_u8(SUPPORTED_GMA_VERSION)?;

	w.write_u64::<LittleEndian>(0u64)?; // steamid
	let unix_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
	w.write_u64::<LittleEndian>(unix_time)?;

	w.write_u8(0)?; // "Required content" (unused)

	w.write_cstr(&file.name)?;
	w.write_cstr(&file.description)?;
	//w.write_cstr(&file.author)?;
	w.write_cstr("Author Name")?;
	w.write_i32::<LittleEndian>(1)?; // addon version (unused)

	// write metadata
	for (i, e) in file.entries.iter().enumerate() {
		w.write_u32::<LittleEndian>((1 + i) as u32)?; // file index
		w.write_cstr(&e.name)?;
		w.write_i64::<LittleEndian>(e.size as i64)?;
		w.write_u32::<LittleEndian>(0)?; // TODO calculate CRC?
	}
	w.write_u32::<LittleEndian>(0)?;

	// write content
	for e in &file.entries {
		w.write_all(e.contents.as_ref().unwrap())?;
	}
	w.write_u32::<LittleEndian>(0)?;

	Ok(())
}