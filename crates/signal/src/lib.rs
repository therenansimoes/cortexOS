/// CortexOS Signal Layer
///
/// Provides signal communication primitives including:
/// - Signal patterns and codebooks
/// - Emitter and receiver abstractions
/// - Channel negotiation
/// - Signal evolution framework
/// - Pattern recognition
/// - Adaptive learning

pub mod codebook;
pub mod emitter;
pub mod error;
pub mod evolution;
pub mod learning;
pub mod negotiation;
pub mod receiver;
pub mod recognition;
pub mod signal;

// Re-export commonly used types
pub use codebook::{Codebook, CodebookEntry, StandardSymbol};
pub use emitter::{ConsoleEmitter, Emitter, MockEmitter};
pub use error::{DecodeError, EmitError, NegotiationError, ReceiveError, SignalError};
pub use evolution::{EvolutionConfig, EvolutionEngine, EvolvedPattern, FitnessMetrics};
pub use learning::{CommunicationOutcome, LearningConfig, LearningStats, LearningStrategy, LearningSystem};
pub use negotiation::{ChannelNegotiator, ChannelQuality};
pub use receiver::{MockReceiver, Receiver};
pub use recognition::{MatchConfidence, RecognitionConfig, RecognitionEngine, RecognizedSignal, SignalTemplate};
pub use signal::{Channel, Pulse, Signal, SignalPattern};
