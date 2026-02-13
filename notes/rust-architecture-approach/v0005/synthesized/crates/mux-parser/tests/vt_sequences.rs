//! Tests for complete VT escape sequence parsing.

use mux_parser::{VtParser, ParserState};

#[test]
fn parse_plain_text() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"Hello");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_cursor_up() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[3A");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_cursor_down() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[5B");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_cursor_forward() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[10C");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_cursor_backward() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[2D");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_cursor_position() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[5;10H");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_sgr_bold() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[1m");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_sgr_reset() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[0m");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_sgr_fg_color() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[31m");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_sgr_bg_color() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[44m");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_sgr_256_fg() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[38;5;196m");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_sgr_rgb_fg() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[38;2;255;128;0m");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_erase_display() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[2J");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_erase_line() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[K");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_scroll_up() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[3S");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_scroll_down() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[2T");
    assert!(!events.is_empty());
}

#[test]
fn parse_osc_title() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b]0;My Window Title\x07");
    assert!(!events.is_empty());
}

#[test]
fn parse_osc_st_terminated() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b]0;Title\x1b\\");
    assert!(!events.is_empty());
}

#[test]
fn parse_esc_decsc() {
    // ESC 7 = Save cursor
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b7");
    assert!(!events.is_empty());
}

#[test]
fn parse_esc_decrc() {
    // ESC 8 = Restore cursor
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b8");
    assert!(!events.is_empty());
}

#[test]
fn parse_cr() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\r");
    assert!(!events.is_empty());
}

#[test]
fn parse_lf() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\n");
    assert!(!events.is_empty());
}

#[test]
fn parse_tab() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\t");
    assert!(!events.is_empty());
}

#[test]
fn parse_backspace() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x08");
    assert!(!events.is_empty());
}

#[test]
fn parse_bell() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x07");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_private_mode() {
    // CSI ? 25 h = show cursor
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[?25h");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_private_mode_hide() {
    // CSI ? 25 l = hide cursor
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[?25l");
    assert!(!events.is_empty());
}

#[test]
fn parse_mixed_text_and_escapes() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"Hello \x1b[1mWorld\x1b[0m!");
    assert!(events.len() >= 3);
}

#[test]
fn parse_utf8_text() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes("Hello 世界".as_bytes());
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_insert_lines() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[3L");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_delete_lines() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[2M");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_insert_chars() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[5@");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_delete_chars() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[3P");
    assert!(!events.is_empty());
}

#[test]
fn parse_csi_set_scroll_region() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"\x1b[1;24r");
    assert!(!events.is_empty());
}

#[test]
fn parse_empty_input() {
    let mut parser = VtParser::new();
    let events = parser.process_bytes(b"");
    assert!(events.is_empty());
}

#[test]
fn parser_state_ground_after_complete_sequence() {
    let mut parser = VtParser::new();
    parser.process_bytes(b"\x1b[1m");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn parser_state_mid_sequence() {
    let mut parser = VtParser::new();
    parser.process_bytes(b"\x1b[");
    assert_ne!(parser.state(), ParserState::Ground);
}

#[test]
fn parser_chunked_input() {
    let mut parser = VtParser::new();
    parser.process_bytes(b"\x1b");
    parser.process_bytes(b"[");
    parser.process_bytes(b"1");
    parser.process_bytes(b"m");
    assert_eq!(parser.state(), ParserState::Ground);
}
