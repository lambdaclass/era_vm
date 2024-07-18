use u256::U256;

use crate::{eravm_error::EraVmError, state::VMState};

pub struct Output {
    pub storage_zero: U256,
    pub vm_state: VMState,
    pub reverted: bool,
    pub reason: Option<EraVmError>,
}
