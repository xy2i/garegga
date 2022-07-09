//! Physical memory allocator.
//! Hands out page frames.

const ADDRESS_SPACE: usize = 0xffff_ffff;
const BITMAP_SLOTS: usize = ADDRESS_SPACE / 8;

/// A bitmap is an array of bits, usable as a set
/// This impl is hardcoded for the PMM. If we use more bitmaps,
/// a more general solution might be needed
pub struct Bitmap {
    data: [u8; BITMAP_SLOTS],
}

impl Bitmap {
    /// Create a bitmap with `BITMAP_SLOTS` slots.
    pub const fn new() -> Self {
        Self {
            data: [0; BITMAP_SLOTS],
        }
    }

    /// Get bit `i`.
    fn get(&self, i: usize) -> bool {
        let byte = i / 8;
        let bit = i % 8;
        self.data[byte] & !(1 << bit) != 0
    }

    /// Set bit `i`.
    fn set(&mut self, i: usize) {
        let byte = i / 8;
        let bit = i % 8;
        self.data[byte] |= 1 << bit;
    }

    /// Unset bit `i`.
    fn unset(&mut self, i: usize) {
        let byte = i / 8;
        let bit = i % 8;
        self.data[byte] &= !(1 << bit);
    }

    /// Get an iterator to each entry of the bitmap.
    fn iter<'a>(&'a self) -> BitmapIter<'a> {
        BitmapIter {
            index: 0,
            bitmap: self,
        }
    }
}

pub struct BitmapIter<'a> {
    index: usize,
    bitmap: &'a Bitmap,
}

impl Iterator for BitmapIter<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == BITMAP_SLOTS - 1 {
            return None;
        }

        let item = self.bitmap.get(self.index);

        self.index += 1;

        Some(item)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    static mut B: Bitmap = Bitmap::new();

    #[test_case]
    fn bitmap_works() {
        log!("test mc test");
        unsafe {
            B.set(1);
            assert_eq!(B.get(1), true);
            B.set(1);
            assert_eq!(B.get(1), true);
            B.unset(1);
            assert_eq!(B.get(1), false);

            B.set(2);
            B.unset(3);
            B.set(4);
            assert_eq!(B.get(2), true);
            assert_eq!(B.get(3), false);
            assert_eq!(B.get(4), true);

            for (i, item) in B.iter().take(9).enumerate() {
                if i == 2 || i == 4 {
                    assert_eq!(item, true);
                } else {
                    assert_eq!(item, false);
                }
            }

            let mut count = 0;

            for _ in B.iter() {
                count += 1;
            }

            assert_eq!(count, BITMAP_SLOTS);
        }
    }
}
