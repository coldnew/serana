//! SSE streaming response handling

use bytes::Bytes;
use futures::stream::Stream;
use reqwest::Response;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Stream of SSE events from an LLM API response
pub struct SseStream {
    bytes_stream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
}

impl SseStream {
    /// Create a new SSE stream from an HTTP response
    pub fn new(response: Response) -> Self {
        Self {
            bytes_stream: Box::pin(response.bytes_stream()),
            buffer: String::new(),
        }
    }

    /// Parse SSE data from accumulated buffer and return next JSON chunk
    fn parse_next(&mut self) -> Option<Result<String, anyhow::Error>> {
        // Find first newline
        let line_end = match self.buffer.find('\n') {
            Some(pos) => pos,
            None => return None,
        };

        let line = self.buffer[..line_end].trim().to_string();
        let processed = line_end + 1;

        // Safely remove processed portion
        if processed <= self.buffer.len() {
            self.buffer = self.buffer[processed..].to_string();
        } else {
            self.buffer.clear();
        }

        if line.is_empty() {
            // Empty line, continue parsing
            return self.parse_next();
        }

        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data == "[DONE]" {
                return None;
            }
            if !data.is_empty() {
                // Return the raw JSON data for caller to parse
                return Some(Ok(data.to_string()));
            }
        } else if line.starts_with(':') {
            // Comment line, skip
            return self.parse_next();
        }

        // Line didn't match expected SSE format; discard and continue
        self.parse_next()
    }
}

impl Stream for SseStream {
    type Item = Result<String, anyhow::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First try to parse from existing buffer
        if let Some(chunk) = self.parse_next() {
            return Poll::Ready(Some(chunk));
        }

        // Need more data
        loop {
            match self.bytes_stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&text);
                    if let Some(chunk) = self.parse_next() {
                        return Poll::Ready(Some(chunk));
                    }
                    // Continue to next chunk
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(e.into())));
                }
                Poll::Ready(None) => {
                    // End of stream, check buffer one last time
                    if let Some(chunk) = self.parse_next() {
                        return Poll::Ready(Some(chunk));
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn parses_sse_chunks() {
        let response_body = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\ndata: [DONE]\n\n";
        let stream = SseStream {
            bytes_stream: Box::pin(futures::stream::iter(vec![Ok(Bytes::from_static(response_body))])),
            buffer: String::new(),
        };
        let chunks: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("Hello"));
        assert!(chunks[1].contains("world"));
    }
}
