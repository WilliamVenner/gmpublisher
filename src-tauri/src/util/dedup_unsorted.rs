// cursed
use std::{collections::HashSet, hash::Hash};

pub fn dedup_unsorted<T: Hash + Eq>(vec: &mut Vec<T>) {
	struct PtrCmp<T: Hash + Eq> {
		ptr: *const T,
	}
	impl<T: Hash + Eq> Hash for PtrCmp<T> {
		fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
			unsafe { (*self.ptr).hash(state) };
		}
	}
	impl<T: Hash + Eq> PartialEq for PtrCmp<T> {
		fn eq(&self, other: &Self) -> bool {
			unsafe { *self.ptr == *other.ptr }
		}
	}
	impl<T: Hash + Eq> Eq for PtrCmp<T> {}

	if vec.len() == 2 {
		if vec[0] == vec[1] {
			vec.truncate(1);
		}
	} else if vec.len() > 2 {
		let mut dedup = HashSet::with_capacity(vec.len());
		let mut i = 0;
		while i != vec.len() {
			if !dedup.insert(PtrCmp { ptr: &vec[i] as *const T }) {
				vec.remove(i);
			} else {
				i += 1;
			}
		}
	}
}
