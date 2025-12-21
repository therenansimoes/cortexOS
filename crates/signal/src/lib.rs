// Placeholder
pub mod codebook;
pub mod emitter;
pub mod error;
pub mod forwarder;
pub mod negotiation;
pub mod receiver;
pub mod routing;
pub mod signal;

pub use codebook::{Codebook, CodebookEntry, StandardSymbol};
pub use emitter::{ConsoleEmitter, Emitter, MockEmitter};
pub use error::{
    DecodeError, EmitError, NegotiationError, ReceiveError, RoutingError, SignalError,
};
pub use forwarder::{ForwardedMessage, SignalForwarder};
pub use negotiation::{ChannelNegotiator, ChannelQuality};
pub use receiver::{MockReceiver, Receiver};
pub use routing::{MultiHopRouter, Route, RouteQuality, RoutingTable};
pub use signal::{Channel, Pulse, Signal, SignalPattern};
