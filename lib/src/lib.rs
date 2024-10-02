use reth_primitives::Header;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ProgramInput {
    pub header_chain: Vec<Header>,
}
