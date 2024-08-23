use super::tracer::Tracer;

#[derive(Default)]
pub struct NoTracer {}

impl Tracer for NoTracer {}
