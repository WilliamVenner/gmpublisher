use std::{io::{BufRead, BufWriter, ErrorKind, Seek, SeekFrom, Write}, sync::Arc};

use byteorder::WriteBytesExt;

pub fn stream_len<F: Seek + ?Sized>(f: &mut F) -> Result<u64, std::io::Error> {
	let old_pos = f.stream_position()?;
	let len = f.seek(SeekFrom::End(0))?;
	if old_pos != len {
		f.seek(SeekFrom::Start(old_pos))?;
	}

	Ok(len)
}

pub fn stream_bytes<R: BufRead + ?Sized, W: Write>(r: &mut R, w: &mut BufWriter<W>, mut bytes: usize) -> Result<(), std::io::Error> {
	Ok({
		let consumed = loop {
			let consumed = match r.fill_buf() {
				Ok([]) => break 0,
				Ok(data) if data.len() >= bytes => {
					w.write_all(&data[..bytes])?;
					break bytes;
				}
				Ok(data) => {
					w.write_all(data)?;
					bytes -= data.len();
					data.len()
				}
				Err(e) if e.kind() == ErrorKind::Interrupted => 0,
				Err(e) => return Err(e),
			};
			r.consume(consumed);
		};
		r.consume(consumed);
	})
}

pub trait NTStringReader: BufRead + Seek {
	fn read_nt_string(&mut self) -> Result<String, std::io::Error> {
		let mut buf = vec![];
		let bytes_read = self.read_until(0, &mut buf)?;
		let nt_string = &buf[0..bytes_read - 1];

		Ok(match std::str::from_utf8(nt_string) {
			Ok(str) => str.to_owned(),
			Err(_) => {
				// Some file paths aren't UTF-8 encoded, usually due to Windows NTFS
				// This will simply guess the text encoding and decode it with that instead
				let mut decoder = chardetng::EncodingDetector::new();
				decoder.feed(nt_string, true);
				let encoding = decoder.guess(None, false);
				let (str, _, _) = encoding.decode(nt_string);
				str.to_string()
			}
		})
	}

	fn skip_nt_string(&mut self) -> Result<usize, std::io::Error> {
		let mut buf = vec![];
		self.read_until(0, &mut buf)
	}
}

pub trait NTStringWriter: Write {
	fn write_nt_string<S: AsRef<str>>(&mut self, str: S) -> Result<(), std::io::Error> {
		self.write(str.as_ref().as_bytes())?;
		self.write_u8(0)?;
		Ok(())
	}
}

#[derive(derive_more::Deref, derive_more::DerefMut, Clone, Debug)]
pub struct ArcBytes(Arc<Vec<u8>>);
impl AsRef<[u8]> for ArcBytes {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}
impl From<Vec<u8>> for ArcBytes {
    fn from(bytes: Vec<u8>) -> Self {
        ArcBytes(Arc::new(bytes))
    }
}
