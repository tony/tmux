#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelCommand {
    SplitVertical,
    SplitHorizontal,
    KillPane,
    NextWindow,
    PrevWindow,
    CopyMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_cloneable() {
        let a = KernelCommand::SplitVertical;
        let b = a.clone();
        assert_eq!(a, b);
    }
}
