pub mod anthropic;
pub mod auxiliary;
pub mod credential;
pub mod fallback;
pub mod openai;
pub mod registry;
pub mod streaming;

pub use anthropic::AnthropicClient;
pub use auxiliary::{AuxiliaryBuilder, AuxiliaryClient, AuxiliaryConfig, AuxiliaryTask};
pub use credential::{CredentialProvider, EnvCredential, RefreshableClient, StaticCredential};
pub use fallback::{FallbackChain, FallbackConfig, ProviderEntry, ProviderStatus};
pub use openai::OpenAiClient;
pub use registry::{ModelRole, ProviderRegistry, RoutingClient};
pub use streaming::SseStream;
