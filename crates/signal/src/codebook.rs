use std::collections::HashMap;

use cortex_core::SymbolId;
use serde::{Deserialize, Serialize};

use crate::error::SignalError;
use crate::signal::{Pulse, SignalPattern};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardSymbol {
    Ack,
    Nak,
    TaskRequest,
    Beacon,
    Error,
    Ping,
    Pong,
    Ready,
    Busy,
    Shutdown,
}

impl StandardSymbol {
    pub fn to_symbol_id(self) -> SymbolId {
        SymbolId::from_bytes(self.as_str().as_bytes())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StandardSymbol::Ack => "ACK",
            StandardSymbol::Nak => "NAK",
            StandardSymbol::TaskRequest => "TASK_REQUEST",
            StandardSymbol::Beacon => "BEACON",
            StandardSymbol::Error => "ERROR",
            StandardSymbol::Ping => "PING",
            StandardSymbol::Pong => "PONG",
            StandardSymbol::Ready => "READY",
            StandardSymbol::Busy => "BUSY",
            StandardSymbol::Shutdown => "SHUTDOWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebookEntry {
    pub symbol: SymbolId,
    pub pattern: SignalPattern,
    pub description: Option<String>,
    pub version: u32,
}

impl CodebookEntry {
    pub fn new(symbol: SymbolId, pattern: SignalPattern) -> Self {
        Self {
            symbol,
            pattern,
            description: None,
            version: 1,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Codebook {
    entries: HashMap<SymbolId, CodebookEntry>,
    reverse: HashMap<Vec<u8>, SymbolId>,
    version: u32,
}

impl Default for Codebook {
    fn default() -> Self {
        Self::new()
    }
}

impl Codebook {
    pub fn new() -> Self {
        let mut codebook = Self {
            entries: HashMap::new(),
            reverse: HashMap::new(),
            version: 1,
        };
        codebook.register_standard_symbols();
        codebook
    }

    fn register_standard_symbols(&mut self) {
        let standard_patterns = [
            (StandardSymbol::Ack, vec![Pulse::on(100), Pulse::off(100)]),
            (
                StandardSymbol::Nak,
                vec![Pulse::on(100), Pulse::off(100), Pulse::on(100), Pulse::off(100)],
            ),
            (
                StandardSymbol::TaskRequest,
                vec![
                    Pulse::on(200),
                    Pulse::off(100),
                    Pulse::on(200),
                    Pulse::off(100),
                    Pulse::on(200),
                ],
            ),
            (
                StandardSymbol::Beacon,
                vec![Pulse::on(500), Pulse::off(500)],
            ),
            (
                StandardSymbol::Error,
                vec![
                    Pulse::on(50),
                    Pulse::off(50),
                    Pulse::on(50),
                    Pulse::off(50),
                    Pulse::on(50),
                    Pulse::off(50),
                ],
            ),
            (StandardSymbol::Ping, vec![Pulse::on(150), Pulse::off(150)]),
            (
                StandardSymbol::Pong,
                vec![Pulse::on(150), Pulse::off(50), Pulse::on(150)],
            ),
            (
                StandardSymbol::Ready,
                vec![Pulse::on(300), Pulse::off(100), Pulse::on(100)],
            ),
            (
                StandardSymbol::Busy,
                vec![Pulse::on(100), Pulse::off(100), Pulse::on(300)],
            ),
            (StandardSymbol::Shutdown, vec![Pulse::on(1000)]),
        ];

        for (symbol, pulses) in standard_patterns {
            let entry = CodebookEntry::new(symbol.to_symbol_id(), SignalPattern::new(pulses))
                .with_description(format!("Standard signal: {}", symbol.as_str()));
            self.register_entry(entry);
        }
    }

    pub fn register_entry(&mut self, entry: CodebookEntry) {
        let pattern_key = self.pattern_to_key(&entry.pattern);
        self.reverse.insert(pattern_key, entry.symbol);
        self.entries.insert(entry.symbol, entry);
    }

    pub fn propose_symbol(
        &mut self,
        symbol: SymbolId,
        pattern: SignalPattern,
        description: Option<String>,
    ) -> Result<(), SignalError> {
        let pattern_key = self.pattern_to_key(&pattern);
        if self.reverse.contains_key(&pattern_key) {
            return Err(SignalError::InvalidPattern(
                "pattern already registered".into(),
            ));
        }
        if self.entries.contains_key(&symbol) {
            return Err(SignalError::InvalidPattern(
                "symbol already registered".into(),
            ));
        }

        let mut entry = CodebookEntry::new(symbol, pattern);
        entry.description = description;
        self.register_entry(entry);
        self.version += 1;
        Ok(())
    }

    pub fn encode(&self, symbol: SymbolId) -> Result<&SignalPattern, SignalError> {
        self.entries
            .get(&symbol)
            .map(|e| &e.pattern)
            .ok_or_else(|| SignalError::UnknownSymbol(format!("{:?}", symbol)))
    }

    pub fn decode(&self, pattern: &SignalPattern) -> Result<SymbolId, SignalError> {
        let pattern_key = self.pattern_to_key(pattern);
        self.reverse
            .get(&pattern_key)
            .copied()
            .ok_or_else(|| SignalError::InvalidPattern("unknown pattern".into()))
    }

    pub fn get_entry(&self, symbol: SymbolId) -> Option<&CodebookEntry> {
        self.entries.get(&symbol)
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn pattern_to_key(&self, pattern: &SignalPattern) -> Vec<u8> {
        pattern
            .pulses
            .iter()
            .flat_map(|p| {
                let mut v = vec![if p.on { 1u8 } else { 0u8 }];
                v.extend_from_slice(&p.duration_us.to_le_bytes());
                v
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_symbols() {
        let codebook = Codebook::new();
        assert!(codebook.entry_count() >= 10);

        let ack_pattern = codebook.encode(StandardSymbol::Ack.to_symbol_id()).unwrap();
        assert!(!ack_pattern.pulses.is_empty());
    }

    #[test]
    fn test_encode_decode() {
        let codebook = Codebook::new();
        let symbol = StandardSymbol::Beacon.to_symbol_id();

        let pattern = codebook.encode(symbol).unwrap();
        let decoded = codebook.decode(pattern).unwrap();

        assert_eq!(symbol, decoded);
    }

    #[test]
    fn test_propose_symbol() {
        let mut codebook = Codebook::new();
        let custom_symbol = SymbolId::from_bytes(b"CUSTOM_SIGNAL");
        let custom_pattern = SignalPattern::new(vec![
            Pulse::on(250),
            Pulse::off(250),
            Pulse::on(250),
        ]);

        codebook
            .propose_symbol(custom_symbol, custom_pattern.clone(), Some("Custom test signal".into()))
            .unwrap();

        let encoded = codebook.encode(custom_symbol).unwrap();
        assert_eq!(encoded, &custom_pattern);
    }
}
