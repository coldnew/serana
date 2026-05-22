pub mod auxiliary;
pub mod credential;
pub mod anthropic;
pub mod fallback;
pub mod openai;
pub mod streaming;
pub mod registry;


pub use auxiliary::{AuxiliaryBuilder, AuxiliaryClient, AuxiliaryConfig, AuxiliaryTask};
pub use anthropic::AnthropicClient;
pub use credential::{CredentialProvider, EnvCredential, RefreshableClient, StaticCredential};
pub use fallback::{FallbackChain, FallbackConfig, ProviderEntry, ProviderStatus};
pub use openai::OpenAiClient;
pub use streaming::SseStream;
pub use registry::{ModelRole, ProviderRegistry, RoutingClient};
