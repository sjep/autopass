
#[derive(Debug)]
pub struct Bitmap {
    buf: Vec<u8>,
    size: usize
}

impl Bitmap {
    pub fn new(size: usize) -> Self {
        if size == 0 {
            return Self {
                buf: vec![],
                size
            }
        }
        let buf = vec![0; size / 8 + 1];
        Self {
            buf,
            size
        }
    }

    pub fn set(&mut self, val: usize) {
        assert!(val < self.size);
        self.buf[val / 8] |= (1 << (val % 8)) as u8;
    }

    pub fn unset(&mut self, val: usize) {
        assert!(val < self.size);
        self.buf[val / 8] &= !(1 << (val % 8)) as u8;
    }
    
    pub fn check_set(&self, val: usize) -> bool {
        assert!(val < self.size);
        (self.buf[val / 8] & (1 << (val % 8))) != 0
    }

    pub fn raw_buffer(&self) -> &[u8] {
        &self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap() {
        let mut bmp = Bitmap::new(100);
        assert!(!bmp.check_set(7));
        bmp.set(7);
        assert!(bmp.buf[0] == 128);
        assert!(bmp.check_set(7));

        assert!(!bmp.check_set(8));
        bmp.set(8);
        assert!(bmp.buf[1] == 1);
        assert!(bmp.check_set(8));

        bmp.unset(7);
        assert!(bmp.buf[0] == 0);
        assert!(!bmp.check_set(7));

        bmp.unset(8);
        assert!(bmp.buf[1] == 0);
        assert!(!bmp.check_set(8));
    }
}