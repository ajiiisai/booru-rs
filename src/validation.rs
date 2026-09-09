//! Tag validation for booru queries.
//!
//! This module provides validation for tag names to catch common mistakes
//! before making API requests.
//!
//! # Example
//!
//! ```
//! use booru_rs::validation::{validate_tag, TagValidation};
//!
//! // Valid tags pass
//! assert!(validate_tag("cat_ears").is_valid);
//! assert!(validate_tag("score:>10").is_valid);
//!
//! // Invalid tags return warnings or errors
//! let result = validate_tag("cat ears");  // Space should be underscore
//! assert!(result.has_warnings());
//! ```

use crate::error::{BooruError, Result};
use std::borrow::Cow;

/// Result of validating a tag.
///
/// New fields may be added in minor releases. Read the fields you need
/// instead of destructuring exhaustively.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TagValidation {
    /// The original tag.
    pub original: String,
    /// The normalized/fixed tag, if applicable.
    pub normalized: Option<String>,
    /// Warnings about potential issues.
    pub warnings: Vec<TagWarning>,
    /// Whether the tag is valid for use.
    pub is_valid: bool,
}

impl TagValidation {
    /// Returns true if validation produced any warnings.
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Gets the tag to use (normalized if available, otherwise original).
    #[must_use]
    pub fn tag(&self) -> &str {
        self.normalized.as_ref().unwrap_or(&self.original)
    }
}

/// Warning types for tag validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagWarning {
    /// Tag contains spaces that should be underscores.
    SpacesFound {
        /// Original tag with spaces.
        original: String,
        /// Suggested replacement with underscores.
        suggested: String,
    },
    /// Tag has leading/trailing whitespace.
    LeadingTrailingWhitespace,
    /// Tag is empty.
    EmptyTag,
    /// Tag contains consecutive underscores.
    ConsecutiveUnderscores,
    /// Tag is very long (might hit URL limits).
    VeryLongTag {
        /// Length of the tag.
        length: usize,
    },
    /// Tag contains non-standard characters outside alphanumerics and
    /// printable ASCII punctuation.
    UnusualCharacters {
        /// Characters found that are unusual.
        chars: Vec<char>,
    },
    /// Meta tag may not work on all boorus.
    UnsupportedMetaTag {
        /// The meta tag prefix.
        prefix: String,
    },
}

impl std::fmt::Display for TagWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagWarning::SpacesFound {
                original,
                suggested,
            } => {
                write!(
                    f,
                    "Tag contains spaces: '{}'. Did you mean '{}'?",
                    original, suggested
                )
            }
            TagWarning::LeadingTrailingWhitespace => {
                write!(f, "Tag has leading or trailing whitespace")
            }
            TagWarning::EmptyTag => write!(f, "Tag is empty"),
            TagWarning::ConsecutiveUnderscores => write!(f, "Tag contains consecutive underscores"),
            TagWarning::VeryLongTag { length } => {
                write!(f, "Tag is very long ({} chars), may cause issues", length)
            }
            TagWarning::UnusualCharacters { chars } => {
                write!(f, "Tag contains unusual characters: {:?}", chars)
            }
            TagWarning::UnsupportedMetaTag { prefix } => {
                write!(
                    f,
                    "Meta tag '{}:' may not be supported on all booru sites",
                    prefix
                )
            }
        }
    }
}

/// Known meta tag prefixes that work on most boorus.
const COMMON_META_TAGS: &[&str] = &[
    "rating", "score", "order", "sort", "user", "height", "width", "id", "md5", "source", "parent",
    "pool",
];

/// Meta tags that are specific to certain boorus.
const DANBOORU_ONLY_META_TAGS: &[&str] = &[
    "pixiv_id",
    "favcount",
    "gentags",
    "arttags",
    "chartags",
    "copytags",
    "approver",
    "commenter",
    "noter",
    "flagger",
];

/// Validates a single tag and returns a validation result.
///
/// This helper is advisory. It may suggest a normalized tag, but provider
/// query builders apply their own literal-tag rules.
///
/// This function checks for common mistakes like:
/// - Spaces instead of underscores
/// - Empty tags
/// - Unusual characters
/// - Very long tags that might cause URL length issues
///
/// # Example
///
/// ```
/// use booru_rs::validation::validate_tag;
///
/// let result = validate_tag("cat_ears");
/// assert!(result.is_valid);
/// assert!(!result.has_warnings());
///
/// let result = validate_tag("cat ears");  // Space instead of underscore
/// assert!(result.has_warnings());
/// ```
#[must_use]
pub fn validate_tag(tag: &str) -> TagValidation {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return TagValidation {
            original: tag.to_string(),
            normalized: None,
            warnings: vec![TagWarning::EmptyTag],
            is_valid: false,
        };
    }

    let mut warnings = Vec::new();
    let mut normalized = None;

    // Check for leading/trailing whitespace
    if trimmed != tag {
        warnings.push(TagWarning::LeadingTrailingWhitespace);
        normalized = Some(trimmed.to_string());
    }

    let working_tag = trimmed;

    // Check for spaces that should be underscores
    if working_tag.contains(' ') {
        let suggested = working_tag.replace(' ', "_");
        warnings.push(TagWarning::SpacesFound {
            original: working_tag.to_string(),
            suggested: suggested.clone(),
        });
        normalized = Some(suggested);
    }

    // Check for consecutive underscores
    if working_tag.contains("__") {
        warnings.push(TagWarning::ConsecutiveUnderscores);
    }

    // Check for very long tags
    if working_tag.len() > 100 {
        warnings.push(TagWarning::VeryLongTag {
            length: working_tag.len(),
        });
    }

    let unusual: Vec<char> = working_tag
        .chars()
        .filter(|c| *c != ' ' && !c.is_alphanumeric() && !c.is_ascii_punctuation())
        .collect();

    if !unusual.is_empty() {
        warnings.push(TagWarning::UnusualCharacters { chars: unusual });
    }

    // Check for meta tags
    if let Some(colon_pos) = working_tag.find(':') {
        let prefix = &working_tag[..colon_pos];
        if !COMMON_META_TAGS.contains(&prefix) && DANBOORU_ONLY_META_TAGS.contains(&prefix) {
            warnings.push(TagWarning::UnsupportedMetaTag {
                prefix: prefix.to_string(),
            });
        }
    }

    TagValidation {
        original: tag.to_string(),
        normalized,
        warnings,
        is_valid: true,
    }
}

/// Validates a tag, returning a normalized value for advisory warnings.
///
/// "Strict" rejects fatal validation results. It may still return a normalized
/// tag for advisory issues such as leading whitespace or spaces between words.
///
/// # Errors
///
/// Returns [`BooruError::InvalidTag`] if the tag is empty or has critical issues.
///
/// # Example
///
/// ```
/// use booru_rs::validation::validate_tag_strict;
///
/// // Valid tags pass through
/// assert!(validate_tag_strict("cat_ears").is_ok());
///
/// // Invalid tags return errors
/// assert!(validate_tag_strict("").is_err());
/// ```
pub fn validate_tag_strict(tag: &str) -> Result<Cow<'_, str>> {
    let result = validate_tag(tag);

    if !result.is_valid {
        return Err(BooruError::InvalidTag {
            tag: tag.to_string(),
            reason: result
                .warnings
                .first()
                .map(|w| w.to_string())
                .unwrap_or_else(|| "Unknown validation error".to_string()),
        });
    }

    if let Some(normalized) = result.normalized {
        Ok(Cow::Owned(normalized))
    } else {
        Ok(Cow::Borrowed(tag))
    }
}

/// Validates multiple tags and returns all normalized.
///
/// # Errors
///
/// Returns [`BooruError::InvalidTag`] if any tag is invalid.
pub fn validate_tags<'a, I>(tags: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = &'a str>,
{
    tags.into_iter()
        .map(|tag| validate_tag_strict(tag).map(|t| t.into_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_tag() {
        let result = validate_tag("cat_ears");
        assert!(result.is_valid);
        assert!(!result.has_warnings());
        assert!(result.normalized.is_none());
    }

    #[test]
    fn test_parenthesized_character_tag_is_not_unusual() {
        let result = validate_tag("belle_(zenless_zone_zero)");
        assert!(result.is_valid);
        assert!(!result.has_warnings());
        assert!(result.normalized.is_none());
    }

    #[test]
    fn test_printable_punctuation_is_not_unusual() {
        for tag in ["d'arc", "k-on!", "foo+bar", "name@example"] {
            let result = validate_tag(tag);
            assert!(result.is_valid);
            assert!(
                !result
                    .warnings
                    .iter()
                    .any(|warning| matches!(warning, TagWarning::UnusualCharacters { .. }))
            );
        }
    }

    #[test]
    fn test_non_printable_and_non_ascii_punctuation_is_unusual() {
        for (tag, expected) in [
            ("foo\tbar", '\t'),
            ("foo\u{7f}bar", '\u{7f}'),
            ("foo—bar", '—'),
        ] {
            let result = validate_tag(tag);
            assert!(matches!(
                result.warnings.iter().find_map(|warning| match warning {
                    TagWarning::UnusualCharacters { chars } => Some(chars),
                    _ => None,
                }),
                Some(chars) if chars == &vec![expected]
            ));
        }
    }

    #[test]
    fn test_spaces_to_underscores() {
        let result = validate_tag("cat ears");
        assert!(result.is_valid);
        assert!(result.has_warnings());
        assert_eq!(result.normalized, Some("cat_ears".to_string()));
    }

    #[test]
    fn test_empty_tag() {
        let result = validate_tag("");
        assert!(!result.is_valid);
        assert!(matches!(
            result.warnings.first(),
            Some(TagWarning::EmptyTag)
        ));
    }

    #[test]
    fn test_whitespace_only_tag_is_invalid() {
        for tag in [" ", "\t", " \n ", "\u{a0}"] {
            let result = validate_tag(tag);
            assert!(!result.is_valid);
            assert_eq!(result.normalized, None);
            assert!(matches!(
                result.warnings.first(),
                Some(TagWarning::EmptyTag)
            ));
            assert!(validate_tag_strict(tag).is_err());
        }
    }

    #[test]
    fn test_leading_trailing_whitespace() {
        let result = validate_tag("  cat_ears  ");
        assert!(result.is_valid);
        assert!(result.has_warnings());
        assert_eq!(result.normalized, Some("cat_ears".to_string()));
    }

    #[test]
    fn test_meta_tag() {
        let result = validate_tag("rating:general");
        assert!(result.is_valid);
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_danbooru_specific_meta_tag() {
        let result = validate_tag("pixiv_id:12345");
        assert!(result.is_valid);
        assert!(result.has_warnings());
        assert!(matches!(
            result.warnings.first(),
            Some(TagWarning::UnsupportedMetaTag { .. })
        ));
    }

    #[test]
    fn test_validate_tag_strict() {
        assert!(validate_tag_strict("cat_ears").is_ok());
        assert!(validate_tag_strict("").is_err());

        // Spaces get normalized
        let result = validate_tag_strict("cat ears").unwrap();
        assert_eq!(result.as_ref(), "cat_ears");
    }
}
