use std::sync::Mutex;

/// Destino de eventos do core (implementado pela camada Tauri e, em testes, por `MemorySink`).
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

/// Sink que guarda os eventos em memória, para testes.
#[derive(Debug, Default)]
pub struct MemorySink {
    events: Mutex<Vec<(String, serde_json::Value)>>,
}

impl MemorySink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<(String, serde_json::Value)> {
        self.events.lock().unwrap().clone()
    }

    pub fn named(&self, event: &str) -> Vec<serde_json::Value> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name == event)
            .map(|(_, payload)| payload.clone())
            .collect()
    }
}

impl EventSink for MemorySink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        self.events
            .lock()
            .unwrap()
            .push((event.to_string(), payload));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registra_eventos_em_ordem() {
        let sink = MemorySink::new();
        sink.emit("a", json!({"n": 1}));
        sink.emit("b", json!(2));
        sink.emit("a", json!({"n": 3}));

        assert_eq!(sink.events().len(), 3);
        assert_eq!(sink.named("a"), vec![json!({"n": 1}), json!({"n": 3})]);
        assert_eq!(sink.named("b"), vec![json!(2)]);
    }

    #[test]
    fn funciona_como_trait_object() {
        let sink = MemorySink::new();
        let dyn_sink: &dyn EventSink = &sink;
        dyn_sink.emit("x", json!(null));
        assert_eq!(sink.events().len(), 1);
    }
}
