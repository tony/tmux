// Parses commands like "new-session -s foo"

#[derive(Debug)]
pub enum Command {
    NewSession { name: Option<String> },
    KillServer,
    // ...
}

pub fn parse_command(input: &str) -> Result<Command, ()> {
    todo!()
}
--- END FILE ---
