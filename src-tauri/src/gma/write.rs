use std::io::{BufWriter, Seek, Write};

pub struct GMAWriteHandle<W: Write + Seek> {
	pub inner: BufWriter<W>
}
impl<W: Write + Seek> std::ops::Deref for GMAWriteHandle<W> {
    type Target = BufWriter<W>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<W: Write + Seek> std::ops::DerefMut for GMAWriteHandle<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}