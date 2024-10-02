//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use log0_summer_lib::ProgramInput;

pub fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a custom system call which handles reading inputs
    // from the prover.
    let input = sp1_zkvm::io::read::<ProgramInput>();

    let start_header = &input.header_chain[0].parent_hash;
    let end_header = input
        .header_chain
        .iter()
        .fold(start_header.clone(), |prev_hash, header| {
            assert_eq!(prev_hash.as_slice(), header.parent_hash.as_slice());
            header.hash_slow()
        });

    let mut final_output = [0; 64];

    final_output[0..32].copy_from_slice(start_header.as_slice());
    final_output[32..64].copy_from_slice(end_header.as_slice());

    sp1_zkvm::io::commit_slice(&final_output);
}
