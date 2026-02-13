// Handles #{session_name} expansion

pub struct FormatString(String);

impl FormatString {
    pub fn parse(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn expand<C>(&self, context: &C) -> String {
        todo!()
    }
}
--- END FILE ---
