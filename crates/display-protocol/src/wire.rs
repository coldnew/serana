use super::{DisplayCmd, FrameUpdate, InputEvent};
use serde::{Deserialize, Serialize};

/// Messages sent over the wire between mora server and client.
///
/// Protocol is newline-delimited JSON (NDJSON) over TCP.
/// Each message is a JSON object followed by `\n`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireMessage {
    /// Server sends on client connect
    ServerHello {
        version: u32,
        width: u16,
        height: u16,
    },
    /// Server sends a rendered frame
    Frame(FrameUpdate),
    /// Server sends an imperative command
    Cmd(DisplayCmd),
    /// Client sends on connect
    ClientHello { version: u32 },
    /// Client sends input events
    Input(InputEvent),
}

impl WireMessage {
    /// Serialize to a JSON line (with trailing newline)
    pub fn to_json_line(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = serde_json::to_vec(self)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Deserialize from a JSON line (without trailing newline)
    pub fn from_json_line(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

pub const PROTOCOL_VERSION: u32 = 1;
