#![forbid(unsafe_code)]

pub const CONTROL_BOUND: usize = 512;
pub const DATA_BOUND: usize = 1024;
pub const DATA_FRAME_BYTES: usize = 64 * 1024;
pub const RENDER_BOUND: usize = 256;
pub const EFFECT_BOUND: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    Control,
    Data,
    Render,
    Effect,
}

#[must_use]
pub const fn lane_bound(lane: Lane) -> usize {
    match lane {
        Lane::Control => CONTROL_BOUND,
        Lane::Data => DATA_BOUND,
        Lane::Render => RENDER_BOUND,
        Lane::Effect => EFFECT_BOUND,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_bound_matches_spec() { assert_eq!(CONTROL_BOUND, 512); }

    #[test]
    fn data_bound_matches_spec() { assert_eq!(DATA_BOUND, 1024); }

    #[test]
    fn data_frame_size_matches_spec() { assert_eq!(DATA_FRAME_BYTES, 64 * 1024); }

    #[test]
    fn render_bound_matches_spec() { assert_eq!(RENDER_BOUND, 256); }

    #[test]
    fn effect_bound_matches_spec() { assert_eq!(EFFECT_BOUND, 1024); }

    #[test]
    fn lane_bound_lookup() { assert_eq!(lane_bound(Lane::Render), 256); }
}
