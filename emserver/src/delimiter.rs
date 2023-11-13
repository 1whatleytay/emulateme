#[derive(Default)]
pub struct Delimiter {
    size: Option<u64>,
    buffer: Vec<u8>
}

impl Delimiter {
    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data)
    }

    pub fn pop(&mut self) -> Option<Vec<u8>> {
        if self.size.is_none() && self.buffer.len() >= 8 {
            let size = u64::from_be_bytes((&self.buffer[0 .. 8]).try_into().unwrap());

            self.buffer.drain(0 .. 8);

            self.size = Some(size)
        }

        let size = self.size? as usize;

        if self.buffer.len() >= size {
            self.size = None;

            Some(self.buffer.drain(0..size).collect())
        } else {
            None
        }
    }
}