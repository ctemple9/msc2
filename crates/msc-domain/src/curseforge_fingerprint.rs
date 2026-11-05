//! CurseForge candidate fingerprints use whitespace-filtered MurmurHash2,
//! seed 1. This is NOT an integrity hash; verify provider SHA-1 separately.
//! Mixing follows Austin Appleby's public-domain MurmurHash2 reference:
//! https://github.com/aappleby/smhasher/blob/master/src/MurmurHash2.cpp

pub fn ignored(byte: u8) -> bool {
    matches!(byte, 9 | 10 | 13 | 32)
}

/// Knowing the filtered length first permits a bounded-memory second pass.
pub struct Fingerprint {
    hash: u32,
    word: u32,
    word_bytes: u32,
    expected: u32,
    received: u64,
}
impl Fingerprint {
    pub fn new(filtered_length: u32) -> Self {
        Self {
            hash: 1 ^ filtered_length,
            word: 0,
            word_bytes: 0,
            expected: filtered_length,
            received: 0,
        }
    }
    pub fn update(&mut self, bytes: &[u8]) {
        const MIX: u32 = 0x5bd1e995;
        for &byte in bytes.iter().filter(|&&byte| !ignored(byte)) {
            self.received += 1;
            self.word |= u32::from(byte) << (8 * self.word_bytes);
            self.word_bytes += 1;
            if self.word_bytes == 4 {
                let mut word = self.word.wrapping_mul(MIX);
                word ^= word >> 24;
                word = word.wrapping_mul(MIX);
                self.hash = self.hash.wrapping_mul(MIX) ^ word;
                self.word = 0;
                self.word_bytes = 0;
            }
        }
    }
    pub fn finish(mut self) -> Result<u32, &'static str> {
        const MIX: u32 = 0x5bd1e995;
        if self.received != u64::from(self.expected) {
            return Err("File changed between fingerprint passes");
        }
        if self.word_bytes > 0 {
            self.hash ^= self.word;
            self.hash = self.hash.wrapping_mul(MIX);
        }
        self.hash ^= self.hash >> 13;
        self.hash = self.hash.wrapping_mul(MIX);
        self.hash ^= self.hash >> 15;
        Ok(self.hash)
    }
}
