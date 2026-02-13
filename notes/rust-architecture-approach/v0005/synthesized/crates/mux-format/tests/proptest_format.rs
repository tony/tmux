//! Property-based tests for mux-format.

use proptest::prelude::*;
use mux_format::{FormatContext, expand};

proptest! {
    #[test]
    fn literal_text_passes_through(text in "[a-zA-Z0-9 ]{1,50}") {
        // Text without #{} should pass through
        if !text.contains('#') {
            let ctx = FormatContext::new();
            let result = expand(&text, &ctx);
            prop_assert_eq!(result, text);
        }
    }

    #[test]
    fn variable_substitution(
        key in "[a-z_]{1,10}",
        value in "[a-zA-Z0-9]{1,20}",
    ) {
        let mut ctx = FormatContext::new();
        ctx.set(key.clone(), value.clone());
        let template = format!("#{{{key}}} text");
        let result = expand(&template, &ctx);
        prop_assert!(result.contains(&value));
    }

    #[test]
    fn missing_variable_empty(key in "[a-z_]{1,10}") {
        let ctx = FormatContext::new();
        let template = format!("#{{{key}}} end");
        let result = expand(&template, &ctx);
        // Missing variable should produce empty string + " end"
        prop_assert!(result.ends_with(" end"));
    }

    #[test]
    fn multiple_variables(
        k1 in "[a-z]{1,5}",
        v1 in "[A-Z]{1,5}",
        k2 in "[a-z]{1,5}",
        v2 in "[A-Z]{1,5}",
    ) {
        let mut ctx = FormatContext::new();
        ctx.set(k1.clone(), v1.clone());
        ctx.set(k2.clone(), v2.clone());
        let template = format!("#{{{k1}}}-#{{{k2}}}");
        let result = expand(&template, &ctx);
        // Both values should appear
        if k1 != k2 {
            prop_assert!(result.contains(&v1));
            prop_assert!(result.contains(&v2));
        }
    }
}

#[test]
fn empty_template() {
    let ctx = FormatContext::new();
    assert_eq!(expand("", &ctx), "");
}

#[test]
fn simple_variable() {
    let mut ctx = FormatContext::new();
    ctx.set("name", "dev");
    let result = expand("#{name}", &ctx);
    assert_eq!(result, "dev");
}

#[test]
fn surrounded_variable() {
    let mut ctx = FormatContext::new();
    ctx.set("name", "dev");
    let result = expand("[#{name}]", &ctx);
    assert_eq!(result, "[dev]");
}

#[test]
fn special_characters_in_value() {
    let mut ctx = FormatContext::new();
    ctx.set("path", "/home/user/dir");
    let result = expand("#{path}", &ctx);
    assert_eq!(result, "/home/user/dir");
}

#[test]
fn context_get_and_set() {
    let mut ctx = FormatContext::new();
    ctx.set("key", "value");
    assert_eq!(ctx.get("key"), Some("value"));
    assert_eq!(ctx.get("missing"), None);
}
