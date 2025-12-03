use opentelemetry::KeyValue;

pub struct Resource {
    pub instance: String,
    pub attrs: Vec<KeyValue>,
    pub metrics: Vec<Counter>,
}

// TODO: support other metrics
pub struct Counter {
    pub name: String,
    pub attrs: Vec<KeyValue>,
    pub value: u64,
}

impl Resource {
    pub fn new(instance: String) -> Self {
        Self {
            instance,
            attrs: Vec::new(),
            metrics: Vec::new(),
        }
    }
}

impl Counter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            attrs: Vec::new(),
            value: 0,
        }
    }
}