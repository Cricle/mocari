use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelSession {
    models: HashMap<String, ()>,
    next_id: u64,
}

impl ModelSession {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            next_id: 0,
        }
    }
}
