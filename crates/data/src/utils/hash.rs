use sha2::{Digest, Sha256};

pub fn to_sha256(vec: Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(vec);

    // Finalize the hash and return it as a 32-byte array
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

pub fn sha256_to_string(bytes: Vec<u8>) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
}
