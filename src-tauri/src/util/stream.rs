use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};

pub fn stream_len<F: Seek>(f: &mut F) -> Result<u64, std::io::Error> {
	let old_pos = f.stream_position()?;
	let len = f.seek(SeekFrom::End(0))?;
	if old_pos != len {
		f.seek(SeekFrom::Start(old_pos))?;
	}

	Ok(len)
}

pub fn stream_bytes<R: Read, W: Write>(r: &mut BufReader<R>, w: &mut BufWriter<W>, mut bytes: usize) -> Result<(), std::io::Error> {
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
