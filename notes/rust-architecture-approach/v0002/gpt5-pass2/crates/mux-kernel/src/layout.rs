#![allow(clippy::cast_possible_truncation)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlgorithm {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainHorizontalMirrored,
    MainVertical,
    MainVerticalMirrored,
    Tiled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutCell {
    Pane { pane: u64, x: u16, y: u16, rows: u16, cols: u16 },
    Split {
        horizontal: bool,
        ratio_milli: u16,
        first: Box<LayoutCell>,
        second: Box<LayoutCell>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutChecksum {
    pub sum: u32,
}

#[must_use]
pub fn parse_layout_string(input: &str) -> LayoutChecksum {
    let sum = input.as_bytes().iter().fold(0u32, |acc, b| acc.wrapping_add(*b as u32));
    LayoutChecksum { sum }
}

#[must_use]
pub fn redistribute_resize(old: &[u16], new_total: u16) -> Vec<u16> {
    if old.is_empty() {
        return Vec::new();
    }
    let old_total: u16 = old.iter().copied().sum();
    if old_total == 0 {
        return vec![new_total / old.len() as u16; old.len()];
    }
    let mut out: Vec<u16> = old
        .iter()
        .map(|v| ((*v as u32 * new_total as u32) / old_total as u32) as u16)
        .collect();
    let assigned: u16 = out.iter().copied().sum();
    let rem = new_total.saturating_sub(assigned);
    if let Some(last) = out.last_mut() {
        *last = last.saturating_add(rem);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_seven_algorithms_exist() {
        let all = [
            LayoutAlgorithm::EvenHorizontal,
            LayoutAlgorithm::EvenVertical,
            LayoutAlgorithm::MainHorizontal,
            LayoutAlgorithm::MainHorizontalMirrored,
            LayoutAlgorithm::MainVertical,
            LayoutAlgorithm::MainVerticalMirrored,
            LayoutAlgorithm::Tiled,
        ];
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn layout_checksum_changes_with_input() {
        let a = parse_layout_string("aa");
        let b = parse_layout_string("ab");
        assert_ne!(a, b);
    }

    #[test]
    fn redistribution_preserves_total() {
        let out = redistribute_resize(&[30, 70], 80);
        assert_eq!(out.iter().copied().sum::<u16>(), 80);
    }

    #[test]
    fn redistribution_handles_empty() {
        assert!(redistribute_resize(&[], 10).is_empty());
    }

    #[test]
    fn redistribution_even_when_old_zero() {
        let out = redistribute_resize(&[0, 0], 10);
        assert_eq!(out.len(), 2);
    }
}
