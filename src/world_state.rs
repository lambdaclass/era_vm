use u256::{H160, U256};

use crate::store::Storage;

#[derive(Debug)]
pub struct WorldState {
    pub storage: Box<dyn Storage>,
}

impl WorldState {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self { storage }
    }

    pub fn decommit_from_address(&self, contract_address: &H160) -> Vec<U256> {
        self.storage.decommit(contract_address)
    }

    pub fn decommit(&mut self, contract_hash: &U256) -> Vec<U256> {
        // TODO: Do the proper decommit operation
        self.storage
            .get_contract_code(contract_hash)
            .expect("Fatal: contract does not exist")
    }
}
