// Message bus integration (NATS/Redis/Kafka) placeholder.
//
// The intent is that agentic workflows (Go adapter) and optional modules can use
// a bus for coordination.

#[derive(Debug, Clone, Default)]
pub struct Bus;

impl Bus {
    pub fn new() -> Self {
        Self
    }
}

