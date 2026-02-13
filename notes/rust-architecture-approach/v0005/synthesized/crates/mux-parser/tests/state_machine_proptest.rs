//! Property-based tests for the VT parser state machine.

use proptest::prelude::*;
use mux_parser::state_machine::{ParserState, TransitionAction, transition};

proptest! {
    #[test]
    fn esc_byte_always_goes_to_escape(state in arb_state(), _dummy in 0..1u8) {
        let (next, _) = transition(state, 0x1B);
        prop_assert_eq!(next, ParserState::Escape);
    }

    #[test]
    fn cancel_byte_always_goes_to_ground(state in arb_state(), _dummy in 0..1u8) {
        let (next, action) = transition(state, 0x18);
        prop_assert_eq!(next, ParserState::Ground);
        prop_assert_eq!(action, TransitionAction::Execute);
    }

    #[test]
    fn sub_byte_always_goes_to_ground(state in arb_state(), _dummy in 0..1u8) {
        let (next, action) = transition(state, 0x1A);
        prop_assert_eq!(next, ParserState::Ground);
        prop_assert_eq!(action, TransitionAction::Execute);
    }

    #[test]
    fn ground_printable_stays_ground(byte in 0x20..=0x7Eu8) {
        let (next, action) = transition(ParserState::Ground, byte);
        prop_assert_eq!(next, ParserState::Ground);
        prop_assert_eq!(action, TransitionAction::Print);
    }

    #[test]
    fn csi_final_byte_goes_to_ground(byte in 0x40..=0x7Eu8) {
        let (next, action) = transition(ParserState::CsiParam, byte);
        prop_assert_eq!(next, ParserState::Ground);
        prop_assert_eq!(action, TransitionAction::CsiDispatch);
    }

    #[test]
    fn csi_digit_stays_in_param(byte in 0x30..=0x39u8) {
        let (next, action) = transition(ParserState::CsiParam, byte);
        prop_assert_eq!(next, ParserState::CsiParam);
        prop_assert_eq!(action, TransitionAction::Param);
    }

    #[test]
    fn osc_non_terminator_stays(byte in 0x20..=0x7Eu8) {
        let (next, action) = transition(ParserState::OscString, byte);
        prop_assert_eq!(next, ParserState::OscString);
        prop_assert_eq!(action, TransitionAction::OscPut);
    }

    #[test]
    fn ground_c0_execute(byte in prop_oneof![
        Just(0x00u8), Just(0x01u8), Just(0x02u8), Just(0x03u8),
        Just(0x04u8), Just(0x05u8), Just(0x06u8), Just(0x07u8),
        Just(0x08u8), Just(0x09u8), Just(0x0Au8), Just(0x0Bu8),
        Just(0x0Cu8), Just(0x0Du8), Just(0x0Eu8), Just(0x0Fu8),
        Just(0x10u8), Just(0x11u8), Just(0x12u8), Just(0x13u8),
        Just(0x14u8), Just(0x15u8), Just(0x16u8), Just(0x17u8),
        Just(0x19u8), Just(0x1Cu8), Just(0x1Du8), Just(0x1Eu8), Just(0x1Fu8),
    ]) {
        let (next, action) = transition(ParserState::Ground, byte);
        prop_assert_eq!(next, ParserState::Ground);
        prop_assert_eq!(action, TransitionAction::Execute);
    }

    #[test]
    fn dcs_passthrough_data_bytes(byte in 0x20..=0x7Eu8) {
        let (next, action) = transition(ParserState::DcsPassthrough, byte);
        prop_assert_eq!(next, ParserState::DcsPassthrough);
        prop_assert_eq!(action, TransitionAction::Put);
    }

    #[test]
    fn transition_is_deterministic(state in arb_state(), byte in proptest::num::u8::ANY) {
        let (s1, a1) = transition(state, byte);
        let (s2, a2) = transition(state, byte);
        prop_assert_eq!(s1, s2);
        prop_assert_eq!(a1, a2);
    }
}

fn arb_state() -> impl Strategy<Value = ParserState> {
    prop_oneof![
        Just(ParserState::Ground),
        Just(ParserState::Escape),
        Just(ParserState::EscapeIntermediate),
        Just(ParserState::CsiEntry),
        Just(ParserState::CsiParam),
        Just(ParserState::CsiIntermediate),
        Just(ParserState::CsiIgnore),
        Just(ParserState::OscString),
        Just(ParserState::DcsEntry),
        Just(ParserState::DcsParam),
        Just(ParserState::DcsIntermediate),
        Just(ParserState::DcsPassthrough),
        Just(ParserState::DcsIgnore),
        Just(ParserState::SosPmApcString),
    ]
}
