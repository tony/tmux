#![allow(dead_code)]
//! # tmux-sniff
//!
//! Wire protocol inspector for TF01 and tmux native protocols.
//! Captures and decodes frames flowing over Unix domain sockets.
//!
//! ## Usage
//! ```text
//! tmux-sniff listen /tmp/termforge.sock
//! tmux-sniff decode < frame.bin
//! tmux-sniff stats /tmp/termforge.sock
//! ```
//!
//! L6 tool crate.

use clap::{Parser, Subcommand};
use mux_proto::{Frame, FrameType};

/// tmux-sniff: Wire protocol inspector for `TermForge`.
#[derive(Parser, Debug)]
#[command(name = "tmux-sniff", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Listen on a socket and dump decoded frames.
    Listen {
        /// Socket path to monitor.
        socket: String,
        /// Maximum number of frames to capture.
        #[arg(long)]
        limit: Option<usize>,
        /// Output format.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Decode a single frame from stdin or a file.
    Decode {
        /// Input file (or stdin if omitted).
        #[arg(long)]
        file: Option<String>,
    },
    /// Show protocol statistics for a socket.
    Stats {
        /// Socket path.
        socket: String,
    },
}

/// Protocol statistics.
#[derive(Debug, Default, Clone)]
struct ProtoStats {
    total_frames: u64,
    total_bytes: u64,
    by_type: std::collections::HashMap<String, u64>,
    errors: u64,
}

impl ProtoStats {
    /// Create new empty stats.
    fn new() -> Self {
        Self::default()
    }

    /// Record a frame.
    fn record_frame(&mut self, frame: &Frame) {
        self.total_frames += 1;
        self.total_bytes += frame.payload.len() as u64 + 16; // header overhead
        let type_name = format_frame_type(frame.frame_type);
        *self.by_type.entry(type_name).or_insert(0) += 1;
    }

    /// Record an error.
    const fn record_error(&mut self) {
        self.errors += 1;
    }

    /// Average frame size.
    fn avg_frame_size(&self) -> f64 {
        if self.total_frames == 0 {
            return 0.0;
        }
        self.total_bytes as f64 / self.total_frames as f64
    }
}

/// Format a frame type for display.
fn format_frame_type(ft: FrameType) -> String {
    match ft {
        FrameType::Hello => "Hello".to_owned(),
        FrameType::Data => "Data".to_owned(),
        FrameType::Resize => "Resize".to_owned(),
        FrameType::Command => "Command".to_owned(),
        FrameType::CommandResponse => "CmdResponse".to_owned(),
        FrameType::KeyInput => "KeyInput".to_owned(),
        FrameType::Shutdown => "Shutdown".to_owned(),
        FrameType::Error => "Error".to_owned(),
    }
}

/// Format a frame for text output.
fn format_frame_text(frame: &Frame, seq: u64) -> String {
    let type_str = format_frame_type(frame.frame_type);
    let payload_preview = if frame.payload.len() > 40 {
        format!("{}...", String::from_utf8_lossy(&frame.payload[..40]))
    } else {
        String::from_utf8_lossy(&frame.payload).to_string()
    };
    format!(
        "[{seq:>6}] {type_str:<12} len={:<6} payload={payload_preview:?}",
        frame.payload.len()
    )
}

/// Decode raw bytes into a frame (uses mux-proto).
fn decode_bytes(data: &[u8]) -> Result<Frame, String> {
    Frame::decode(data).map_err(|e| format!("failed to decode frame: {e}"))
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Listen {
            socket,
            limit,
            format,
        } => {
            eprintln!(
                "tmux-sniff: listen on {socket} (limit={limit:?}, format={format}) \
                 (scaffold -- not implemented)"
            );
        }
        Commands::Decode { file } => {
            let source = file.as_deref().unwrap_or("stdin");
            eprintln!("tmux-sniff: decode from {source} (scaffold -- not implemented)");
        }
        Commands::Stats { socket } => {
            eprintln!("tmux-sniff: stats for {socket} (scaffold -- not implemented)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frame() -> Frame {
        Frame::new(FrameType::Command, b"list-sessions".to_vec())
    }

    #[test]
    fn format_frame_type_command() {
        assert_eq!(format_frame_type(FrameType::Command), "Command");
    }

    #[test]
    fn format_frame_type_data() {
        assert_eq!(format_frame_type(FrameType::Data), "Data");
    }

    #[test]
    fn format_frame_type_hello() {
        assert_eq!(format_frame_type(FrameType::Hello), "Hello");
    }

    #[test]
    fn format_frame_type_shutdown() {
        assert_eq!(format_frame_type(FrameType::Shutdown), "Shutdown");
    }

    #[test]
    fn format_frame_text_short() {
        let frame = sample_frame();
        let text = format_frame_text(&frame, 1);
        assert!(text.contains("Command"));
        assert!(text.contains("list-sessions"));
    }

    #[test]
    fn format_frame_text_long_payload() {
        let frame = Frame::new(FrameType::Data, vec![b'A'; 100]);
        let text = format_frame_text(&frame, 42);
        assert!(text.contains("..."));
    }

    #[test]
    fn proto_stats_new() {
        let stats = ProtoStats::new();
        assert_eq!(stats.total_frames, 0);
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn proto_stats_record_frame() {
        let mut stats = ProtoStats::new();
        stats.record_frame(&sample_frame());
        assert_eq!(stats.total_frames, 1);
        assert!(stats.total_bytes > 0);
        assert_eq!(stats.by_type.get("Command"), Some(&1));
    }

    #[test]
    fn proto_stats_record_error() {
        let mut stats = ProtoStats::new();
        stats.record_error();
        assert_eq!(stats.errors, 1);
    }

    #[test]
    fn proto_stats_avg_size() {
        let stats = ProtoStats::new();
        let avg = stats.avg_frame_size();
        assert!((avg - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn proto_stats_avg_with_data() {
        let mut stats = ProtoStats::new();
        stats.record_frame(&sample_frame());
        let avg = stats.avg_frame_size();
        assert!(avg > 0.0);
    }

    #[test]
    fn decode_roundtrip() {
        let frame = sample_frame();
        let encoded = frame.encode().unwrap_or_default();
        let decoded = decode_bytes(&encoded);
        assert!(decoded.is_ok());
        let decoded = decoded.unwrap_or_else(|_| Frame::new(FrameType::Error, Vec::new()));
        assert_eq!(decoded.frame_type, FrameType::Command);
        assert_eq!(decoded.payload, b"list-sessions");
    }

    #[test]
    fn decode_invalid_bytes() {
        let result = decode_bytes(&[0, 1, 2]);
        assert!(result.is_err());
    }
}
