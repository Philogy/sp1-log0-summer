// SP1 Magic: Do not remove
#![no_main]
sp1_zkvm::entrypoint!(main);

use reth_primitives::Header;

pub fn main() {
    let length = sp1_zkvm::io::read::<u32>();
    let first_header = sp1_zkvm::io::read::<Header>();
    let start_hash = first_header.parent_hash;
    let mut hash = first_header.hash_slow();
    for _ in 1..length {
        let next_header = sp1_zkvm::io::read::<Header>();
        assert_eq!(hash, next_header.parent_hash);
        hash = next_header.hash_slow();
    }

    let mut final_output = [0; 64];

    final_output[0..32].copy_from_slice(start_hash.as_slice());
    final_output[32..64].copy_from_slice(hash.as_slice());

    sp1_zkvm::io::commit_slice(&final_output);
}
