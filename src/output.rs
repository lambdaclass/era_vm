use u256::U256;

use crate::{eravm_error::EraVmError, execution::Execution};

pub struct Output {
    pub storage_zero: U256,
    pub vm_state: Execution,
    pub reverted: bool,
    pub reason: Option<EraVmError>,
}
