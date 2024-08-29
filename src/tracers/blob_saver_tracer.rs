use super::tracer::Tracer;
use crate::{
    eravm_error::{EraVmError, HeapError},
    execution::Execution,
    value::FatPointer,
    Opcode,
};
use std::collections::HashMap;
use u256::{H160, H256, U256};
use zkevm_opcode_defs::ethereum_types::Address;
use zkevm_opcode_defs::sha2::{Digest, Sha256};

#[derive(Default, Debug)]
pub struct BlobSaverTracer {
    pub blobs: HashMap<U256, Vec<U256>>,
}

impl BlobSaverTracer {
    pub fn new() -> Self {
        Self {
            blobs: Default::default(),
        }
    }
}

const CONTRACT_DEPLOYER_ADDRESS: Address = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x80, 0x06,
]);

const KNOWN_CODES_STORAGE_ADDRESS: Address = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x80, 0x04,
]);

// Hardcoded signature of `publishEVMBytecode` function.
// In hex is 0x964eb607
const PUBLISH_BYTECODE_SIGNATURE: [u8; 4] = [0x96, 0x4e, 0xb6, 0x7];

fn hash_evm_bytecode(bytecode: &[u8]) -> H256 {
    let mut hasher = Sha256::new();
    let len = bytecode.len() as u16;
    hasher.update(bytecode);
    let result = hasher.finalize();

    let mut output = [0u8; 32];
    output[..].copy_from_slice(result.as_slice());
    output[0] = 2; //BLOB version byte
    output[1] = 0;
    output[2..4].copy_from_slice(&len.to_be_bytes());

    H256(output)
}

impl Tracer for BlobSaverTracer {
    fn before_execution(&mut self, _opcode: &Opcode, vm: &mut Execution) -> Result<(), EraVmError> {
        let current_callstack = vm.current_context()?;

        // Here we assume that the only case when PC is 0 at the start of the execution of the contract.
        let known_code_storage_call = current_callstack.code_address == KNOWN_CODES_STORAGE_ADDRESS
            && current_callstack.frame.pc == 0 // FarCall
            && current_callstack.caller == CONTRACT_DEPLOYER_ADDRESS;

        if !known_code_storage_call {
            // Leave
            return Ok(());
        }

        // Now, we need to check whether it is indeed a call to publish EVM code.
        let calldata_ptr = vm.get_register(1);
        if !calldata_ptr.is_pointer {
            return Ok(());
        }

        let ptr = FatPointer::decode(calldata_ptr.value);
        let data = vm
            .heaps
            .get(ptr.page)
            .ok_or(HeapError::ReadOutOfBounds)?
            .read_unaligned_from_pointer(&ptr)?;

        if data.len() < 64 {
            // Not interested
            return Ok(());
        }

        let (signature, data) = data.split_at(4);

        if signature != PUBLISH_BYTECODE_SIGNATURE {
            return Ok(());
        }

        let (_, published_bytecode) = data.split_at(64);

        if published_bytecode.len() % 32 != 0 {
            eprintln!("Invalid bytecode length");
            return Ok(());
        }

        let hash = hash_evm_bytecode(published_bytecode);
        let as_words = published_bytecode
            .chunks(32)
            .map(U256::from_big_endian)
            .collect();

        let key = U256::from_big_endian(hash.as_bytes());

        self.blobs.insert(key, as_words);

        Ok(())
    }
}
