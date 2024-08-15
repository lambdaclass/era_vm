mod address_operands;
pub mod call_frame;
mod eravm_error;
pub mod heaps;
mod op_handlers;
mod opcode;
pub mod output;
mod precompiles;
mod ptr_operator;
pub mod execution;
pub mod store;
pub mod tracers;
pub mod utils;
pub mod value;
pub mod vm;
pub use opcode::Opcode;
pub use execution::Execution;
pub use vm::EraVM;
mod rollbacks;
pub mod state;
use zkevm_opcode_defs::Opcode as Variant;
