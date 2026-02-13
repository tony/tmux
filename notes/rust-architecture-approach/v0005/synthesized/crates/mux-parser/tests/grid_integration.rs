//! Integration tests: parser output into grid cells.

use mux_parser::VtParser;
use mux_parser::VtAction;
use mux_grapheme_arena::GraphemeArena;
use mux_grid::ChunkedGrid;

#[test]
fn parser_output_to_grid() {
    let mut parser = VtParser::new();
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);

    let actions = parser.process_bytes(b"Hello");
    for action in actions {
        if let VtAction::Print(ch) = action {
            let id = arena.intern(&ch.to_string());
            grid.write_char(id, 1);
        }
    }
    assert_eq!(grid.line_text(0, &arena), "Hello");
}

#[test]
fn parser_csi_moves_cursor() {
    let mut parser = VtParser::new();
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);

    let actions = parser.process_bytes(b"AB\x1b[2DC");
    for action in actions {
        match action {
            VtAction::Print(ch) => {
                let id = arena.intern(&ch.to_string());
                grid.write_char(id, 1);
            }
            VtAction::CsiDispatch(params) => {
                if params.final_byte == b'D' {
                    let n = params.get(0, 1);
                    let (x, y) = grid.cursor();
                    grid.set_cursor(x.saturating_sub(n), y);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn parser_newline_feeds_grid() {
    let mut parser = VtParser::new();
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);

    let actions = parser.process_bytes(b"Line1\r\nLine2");
    for action in actions {
        match action {
            VtAction::Print(ch) => {
                let id = arena.intern(&ch.to_string());
                grid.write_char(id, 1);
            }
            VtAction::Execute(0x0A) => grid.line_feed(),
            VtAction::Execute(0x0D) => grid.carriage_return(),
            _ => {}
        }
    }
    assert_eq!(grid.line_text(0, &arena), "Line1");
    assert_eq!(grid.line_text(1, &arena), "Line2");
}

#[test]
fn parser_sgr_applied() {
    let mut parser = VtParser::new();
    let actions = parser.process_bytes(b"\x1b[1;31mRed");
    // Should produce SGR dispatch + 3 print actions
    let csi_count = actions.iter().filter(|a| matches!(a, VtAction::CsiDispatch(_))).count();
    let print_count = actions.iter().filter(|a| matches!(a, VtAction::Print(_))).count();
    assert_eq!(csi_count, 1);
    assert_eq!(print_count, 3);
}
