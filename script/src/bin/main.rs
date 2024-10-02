//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release -- --prove
//! ```

use alloy_provider::{Provider, ProviderBuilder};
use clap::Parser;
use log0_summer_lib::ProgramInput;
use reth_primitives::Header;
use sp1_sdk::{ProverClient, SP1Stdin};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const ELF: &[u8] = include_bytes!("../../../elf/riscv32im-succinct-zkvm-elf");

/// The arguments for the command.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, group = "mode")]
    execute: bool,

    #[clap(long, group = "mode")]
    prove: bool,

    #[clap(long, default_value = "http://localhost:8545")]
    rpc_url: String,

    #[clap(long, help = "start block")]
    start: u64,

    #[clap(long, help = "end block")]
    end: u64,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    let rpc_url = args.rpc_url.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);

    println!("fetching blocks {}-{}", args.start, args.end);

    let blocks = futures::future::try_join_all(
        (args.start..=args.end).map(|block| provider.get_block_by_number(block.into(), false)),
    )
    .await?;
    let headers: Vec<_> = blocks
        .into_iter()
        .map(|block| block.expect("missing block in range").header)
        .map(|header| Header {
            parent_hash: header.parent_hash,
            ommers_hash: header.uncles_hash,
            beneficiary: header.miner,
            state_root: header.state_root,
            transactions_root: header.transactions_root,
            receipts_root: header.receipts_root,
            withdrawals_root: header.withdrawals_root,
            logs_bloom: header.logs_bloom,
            difficulty: header.difficulty,
            number: header.number,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            timestamp: header.timestamp,
            mix_hash: header.mix_hash.unwrap(),
            nonce: header.nonce.unwrap().into(),
            base_fee_per_gas: header.base_fee_per_gas,
            blob_gas_used: header.blob_gas_used,
            excess_blob_gas: header.excess_blob_gas,
            parent_beacon_block_root: header.parent_beacon_block_root,
            requests_root: header.requests_root,
            extra_data: header.extra_data,
        })
        .collect();

    println!("blocks fetched");

    // Setup the prover client.
    let client = ProverClient::new();

    let input = ProgramInput {
        header_chain: headers,
    };

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(ELF, stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let out = output.as_slice();

        let start_header = &out[0..32];
        let end_header = &out[32..64];
        println!("start_header: 0x{}", hex::encode(start_header));
        println!("end_header: 0x{}", hex::encode(end_header));

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(ELF);

        // Generate the proof
        let proof = client
            .prove(&pk, stdin)
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }

    Ok(())
}
