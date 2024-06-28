use u256::{H160, U256};
use zkevm_opcode_defs::{ethereum_types::Address, DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW};

use crate::state::VMState;

impl VMState {
    /// Decommit the contract code from the storage.
    /// This operation should return the contract code from a given contract hash.
    /// The contract hash should be previously stored.
    pub fn decommit(&mut self, contract_hash: &U256) -> Option<Vec<U256>> {
        // self.storage
        //     .decommit(contract_hash)
        //     .expect("Fatal: contract does not exist")
        Some(vec![])
    }
}

/// Used to load code when the VM is not yet initialized.
// pub fn initial_decommit<T: Storage>(world_state: &mut T, address: H160) -> Vec<U256> {
//     let deployer_system_contract_address =
//         Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
//     let code_info = world_state
//         .read(&deployer_system_contract_address)
//         .unwrap_or_default();

//     let mut code_info_bytes = [0; 32];
//     code_info.to_big_endian(&mut code_info_bytes);

//     code_info_bytes[1] = 0;
//     let code_key: U256 = U256::from_big_endian(&code_info_bytes);

//     world_state.decommit(&code_key)
// }

/// Helper function to convert an H160 address into a U256.
/// Used to store the contract hash in the storage.
pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}
