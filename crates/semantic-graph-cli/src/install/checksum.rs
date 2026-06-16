use sha2::{Digest, Sha256};
use std::fmt::Write;

pub struct Checksum;

impl Checksum {
    pub fn sha256_hex(content: &[u8]) -> String {
        let digest = Sha256::digest(content);
        let mut output = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(output, "{byte:02x}");
        }
        output
    }
}
