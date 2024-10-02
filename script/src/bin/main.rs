use alloy_primitives::BloomInput;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::Filter;
use clap::Parser;
use log0_summer_lib::MY_CONTRACT;
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

    #[clap(long, default_value_t = 1_000)]
    chunk_size: u64,
}

struct RangeChunks {
    current: u64,
    end: u64,
    chunk_size: u64,
}

impl Iterator for RangeChunks {
    type Item = std::ops::Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let next_edge = self.end.min(self.current + self.chunk_size);
            let range = self.current..next_edge;
            self.current = next_edge;
            Some(range)
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    let rpc_url = args.rpc_url.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);

    // let filter = Filter::new()
    //     .from_block(args.start)
    //     .to_block(args.end)
    //     .address(MY_CONTRACT);

    // let the_logs = provider.get_logs(&filter).await?;
    let total_blocks = (args.end - args.start) as usize;

    let mut blocks: Vec<_> = Vec::with_capacity(total_blocks);

    println!("args.start: {}", args.start);
    println!("args.end: {}", args.end);

    let chunks = RangeChunks {
        current: args.start,
        end: args.end,
        chunk_size: args.chunk_size,
    };
    for blocks_chunk in chunks {
        println!("fetching chunk {}-{}", blocks_chunk.start, blocks_chunk.end);
        blocks.extend(
            futures::future::try_join_all(
                blocks_chunk.map(|block| provider.get_block_by_number(block.into(), false)),
            )
            .await?,
        );
    }

    // let contract_bloom = BloomInput::Raw(MY_CONTRACT.as_slice()).into();

    // for (number, block) in (args.start..args.end).zip(blocks.iter()) {
    //     let block = block.as_ref().unwrap();
    //     let log_count = the_logs
    //         .iter()
    //         .filter(|log| log.block_number.unwrap() == number)
    //         .count();
    //     let bloom_hit = block.header.logs_bloom.contains(&contract_bloom);
    //     if bloom_hit {
    //         let emoji = if log_count > 0 { '✅' } else { '❌' };
    //         println!("{}: {} ({})", number, log_count, emoji);
    //     }
    // }

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

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();

    println!("headers.len(): {}", headers.len());

    stdin.write(&(headers.len() as u32));
    for header in headers.iter() {
        stdin.write(header);
    }

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
        println!(
            "Cycles per block: {}",
            report.total_instruction_count() / (args.end - args.start)
        );
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
