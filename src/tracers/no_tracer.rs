use super::tracer::Tracer;
use crate::state::VMState;
use crate::{execution::Execution, Opcode};

#[derive(Default)]
pub struct NoTracer {}

impl Tracer for NoTracer {}
