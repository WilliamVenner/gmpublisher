use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Clone)]
pub(crate) struct Base64Image {
	img: Vec<u8>,
	width: u32,
	height: u32,
}

impl Base64Image {
	pub(crate) fn new(img: Vec<u8>, width: u32, height: u32) -> Base64Image {
		Base64Image { img, width, height }
	}
}

impl Serialize for Base64Image {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut png_buf = Vec::new();
		let encoder = image::png::PngEncoder::new(std::io::Write::by_ref(&mut png_buf));
		if encoder
			.encode(&self.img, self.width, self.height, image::ColorType::Rgba8)
			.is_ok()
		{
			serializer.serialize_some(&base64::encode(png_buf))
		} else {
			serializer.serialize_none()
		}
	}
}

impl fmt::Debug for Base64Image {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Base64Image")
			.field("bytes", &self.img.len())
			.field("width", &self.width)
			.field("height", &self.height)
			.field("resolution", &format!("{}px", &self.width * &self.height))
			.finish()
	}
}
