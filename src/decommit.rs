use u256::{H160, U256};
use zkevm_opcode_defs::{ethereum_types::Address, DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW};

use crate::state::VMState;

impl VMState {
    /// Decommit the contract code from the storage.
    /// This operation should return the contract code from a given contract hash.
    /// The contract hash should be previously stored.
    pub fn decommit(&mut self, contract_hash: &U256) -> Vec<U256> {
        // TODO: Do the proper decommit operation
        self.storage
            .get_contract_code(contract_hash)
            .expect("Fatal: contract does not exist")
    }
    pub fn decommit_from_address(&self, contract_address: &H160) -> Vec<U256> {
        let hash = self
            .storage
            .get_contract_hash(contract_address)
            .expect("Fatal: contract does not exist");
        self
            .storage
            .get_contract_code(&hash)
            .expect("Fatal: hash found but it does not have an associated contract")
    }
}

pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}
