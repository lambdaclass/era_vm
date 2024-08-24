use u256::U256;

pub const STORAGE_READ_STORAGE_APPLICATION_CYCLES: usize = 1;
pub const STORAGE_WRITE_STORAGE_APPLICATION_CYCLES: usize = 2;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VmStatistics {
    pub keccak256_cycles: usize,
    pub ecrecover_cycles: usize,
    pub sha256_cycles: usize,
    pub secp255r1_verify_cycles: usize,
    pub code_decommitter_cycles: usize,
    pub storage_application_cycles: usize,
}

impl VmStatistics {
    pub fn decommiter_cycle_from_decommit(&mut self, code_page: &[U256]) {
        self.code_decommitter_cycles += (code_page.len() + 1) / 2
    }
}
