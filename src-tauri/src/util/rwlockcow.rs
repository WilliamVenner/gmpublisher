use std::borrow::Borrow;
use parking_lot::MappedRwLockReadGuard;
use RwLockCow::*;

pub enum RwLockCow<'a, B: ?Sized + 'a>
where
	B: ToOwned,
{
	/// Locked data.
	Locked(MappedRwLockReadGuard<'a, B>),

	/// Borrowed data.
	Borrowed(&'a B),

	/// Owned data.
	Owned(<B as ToOwned>::Owned),
}
impl<B: ?Sized + ToOwned> std::ops::Deref for RwLockCow<'_, B> {
    type Target = B;

    fn deref(&self) -> &B {
        match *self {
            Borrowed(borrowed) => borrowed,
            Owned(ref owned) => owned.borrow(),
            Locked(ref locked) => &*locked,
        }
    }
}
