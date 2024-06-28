use u256::{H160, U256};
use zkevm_opcode_defs::{ethereum_types::Address, DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW};

use crate::{state::VMState, store::GlobalStorage};

impl VMState {
    /// Decommit the contract code from the storage.
    /// This operation should return the contract code from a given contract hash.
    /// The contract hash should be previously stored.
    pub fn decommit(
        &mut self,
        global: &mut dyn GlobalStorage,
        contract_address: U256,
    ) -> Option<U256> {
        let deployer_system_contract_address =
            Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);

        // let mut is_evm = false;

        let mut code_info = {
            let code_info = global
                .read(&deployer_system_contract_address, &contract_address)
                .unwrap();
            let mut code_info_bytes = [0; 32];
            code_info.to_big_endian(&mut code_info_bytes);

            // // Note that EOAs are considered constructed because their code info is all zeroes.
            // let is_constructed = match code_info_bytes[1] {
            //     0 => true,
            //     1 => false,
            //     _ => {
            //         return None;
            //     }
            // };
            // if is_constructed == is_constructor_call {
            //     return None;
            // }

            match code_info_bytes[0] {
                1 => code_info_bytes,
                // 2 => {
                //     is_evm = true;
                //     evm_interpreter_code_hash
                // }

                // // The address aliasing contract implements Ethereum-like behavior of calls to EOAs
                // // returning successfully (and address aliasing when called from the bootloader).
                // _ if code_info == U256::zero() && !is_kernel(address) => default_aa_code_hash,
                _ => return None,
            }
        };

        code_info[1] = 0;
        let code_key: U256 = U256::from_big_endian(&code_info);

        // if !self.decommitted_hashes.as_ref().contains_key(&code_key) {
        //     let code_length_in_words = u16::from_be_bytes([code_info[2], code_info[3]]);
        //     let cost =
        //         code_length_in_words as u32 * zkevm_opcode_defs::ERGS_PER_CODE_WORD_DECOMMITTMENT;
        //     if cost > *gas {
        //         // Unlike all other gas costs, this one is not paid if low on gas.
        //         return None;
        //     }
        //     *gas -= cost;
        //     self.decommitted_hashes.insert(code_key, ());
        // };

        let program = global.decommit(&code_key);
        Some(program)
    }
}

/// Used to load code when the VM is not yet initialized.
pub fn initial_decommit<T: GlobalStorage>(world_state: &mut T, address: H160) -> U256 {
    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let code_info = world_state
        .read(
            &deployer_system_contract_address,
            &address_into_u256(address),
        )
        .unwrap_or_default();

    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    world_state.decommit(&code_key)
}

/// Helper function to convert an H160 address into a U256.
/// Used to store the contract hash in the storage.
pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}
