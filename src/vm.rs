use crate::{store::Storage, VMState};

pub struct LambdaVm {
    pub state: VMState,
    pub storage: Box<dyn Storage>,
}

impl LambdaVm {
    pub fn new(state: VMState, storage: Box<dyn Storage>) -> Self {
        Self { state, storage }
    }
}
