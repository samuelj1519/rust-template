//! Example library exposing [`normalize_whitespace`].

/// Trim surrounding whitespace and collapse each internal run to one space.
///
/// ```
/// use {{crate_name}}::normalize_whitespace;
///
/// assert_eq!(normalize_whitespace("  a \n\t b  "), "a b");
/// ```
#[must_use]
pub fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalize_whitespace;

    #[test]
    fn collapses_internal_whitespace_and_trims_ends() {
        assert_eq!(normalize_whitespace("  a \n\t b  "), "a b");
    }

    #[test]
    fn already_normalized_text_stays_put() {
        assert_eq!(normalize_whitespace("a b"), "a b");
    }

    #[test]
    fn blank_input_is_empty() {
        assert_eq!(normalize_whitespace(" \n\t "), "");
    }
}
