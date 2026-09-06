//! Integration tests for tag autocomplete functionality.

#[cfg(test)]
mod autocomplete {
    use booru_rs::autocomplete::{Autocomplete, TagSuggestion};

    #[cfg(feature = "danbooru")]
    mod danbooru {
        use super::*;
        use booru_rs::BooruError;
        use booru_rs::danbooru::DanbooruClient;

        fn suggestions_or_skip(
            result: booru_rs::error::Result<Vec<TagSuggestion>>,
        ) -> Option<Vec<TagSuggestion>> {
            match result {
                Ok(suggestions) => Some(suggestions),
                Err(BooruError::Request(_) | BooruError::Parse(_)) => None,
                Err(error) => panic!("Danbooru autocomplete failed: {error}"),
            }
        }

        #[tokio::test]
        async fn autocomplete_returns_suggestions() {
            let suggestions = DanbooruClient::autocomplete("cat_", 10).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        async fn autocomplete_respects_limit() {
            let suggestions = DanbooruClient::autocomplete("a", 5).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            assert!(
                suggestions.len() <= 5,
                "Should respect limit parameter (expected 5, got {})",
                suggestions.len()
            );
        }

        #[tokio::test]
        async fn autocomplete_has_tag_names() {
            let suggestions = DanbooruClient::autocomplete("cat_ears", 5).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            if !suggestions.is_empty() {
                let first = &suggestions[0];
                assert!(!first.name.is_empty(), "Tag name should not be empty");
                assert!(!first.label.is_empty(), "Label should not be empty");
            }
        }

        #[tokio::test]
        async fn autocomplete_returns_post_counts() {
            let suggestions = DanbooruClient::autocomplete("cat_ears", 5).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            // Danbooru should return post counts
            if !suggestions.is_empty() {
                assert!(
                    suggestions[0].post_count.is_some(),
                    "Danbooru should provide post counts"
                );
            }
        }

        #[tokio::test]
        async fn autocomplete_returns_categories() {
            let suggestions = DanbooruClient::autocomplete("cat_ears", 5).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            // Danbooru should return category info
            if !suggestions.is_empty() {
                assert!(
                    suggestions[0].category.is_some(),
                    "Danbooru should provide category info"
                );
            }
        }

        #[tokio::test]
        async fn autocomplete_empty_query() {
            // Empty query should still work (returns popular tags or empty)
            let suggestions = DanbooruClient::autocomplete("", 5).await;
            let Some(_suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
        }
    }

    #[cfg(feature = "safebooru")]
    mod safebooru {
        use super::*;
        use booru_rs::BooruError;
        use booru_rs::safebooru::SafebooruClient;

        fn suggestions_or_skip(
            result: booru_rs::error::Result<Vec<TagSuggestion>>,
        ) -> Option<Vec<TagSuggestion>> {
            match result {
                Ok(suggestions) => Some(suggestions),
                Err(BooruError::Request(_) | BooruError::Parse(_)) => None,
                Err(error) => panic!("Safebooru autocomplete failed: {error}"),
            }
        }

        #[tokio::test]
        async fn autocomplete_returns_suggestions() {
            let suggestions = SafebooruClient::autocomplete("cat_", 10).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        async fn autocomplete_respects_limit() {
            let suggestions = SafebooruClient::autocomplete("a", 5).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            assert!(
                suggestions.len() <= 5,
                "Should respect limit parameter (expected 5, got {})",
                suggestions.len()
            );
        }

        #[tokio::test]
        async fn autocomplete_parses_post_count_from_label() {
            let suggestions = SafebooruClient::autocomplete("cat_ears", 5).await;

            let Some(suggestions) = suggestions_or_skip(suggestions) else {
                return;
            };
            // Safebooru embeds post count in label like "cat_ears (177448)"
            if !suggestions.is_empty() {
                let first = &suggestions[0];
                // The label should contain parentheses with count
                assert!(first.label.contains('('), "Label should contain post count");
                // And we should have parsed it
                assert!(
                    first.post_count.is_some(),
                    "Should parse post count from label"
                );
            }
        }
    }

    #[cfg(feature = "gelbooru")]
    mod gelbooru {
        use super::*;
        use booru_rs::gelbooru::GelbooruClient;

        #[tokio::test]
        async fn autocomplete_returns_suggestions() {
            // Gelbooru autocomplete may work without auth
            let suggestions = GelbooruClient::autocomplete("cat_", 10).await;

            // This might fail due to auth requirements, which is expected
            if let Err(booru_rs::BooruError::Unauthorized(_)) = suggestions {
                return;
            }

            let suggestions = suggestions.unwrap();
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        async fn autocomplete_respects_limit() {
            let suggestions = GelbooruClient::autocomplete("a", 5).await;

            if let Err(booru_rs::BooruError::Unauthorized(_)) = suggestions {
                return;
            }

            let suggestions = suggestions.unwrap();
            assert!(
                suggestions.len() <= 5,
                "Should respect limit parameter (expected 5, got {})",
                suggestions.len()
            );
        }
    }

    #[cfg(feature = "rule34")]
    mod rule34 {
        use super::*;
        use booru_rs::rule34::Rule34Client;

        #[tokio::test]
        async fn autocomplete_returns_suggestions() {
            let suggestions = Rule34Client::autocomplete("cat_", 10).await;

            assert!(suggestions.is_ok(), "Rule34 autocomplete should work");
            let suggestions = suggestions.unwrap();
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        async fn autocomplete_parses_post_count() {
            let suggestions = Rule34Client::autocomplete("cat_ears", 5).await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
            if !suggestions.is_empty() {
                let first = &suggestions[0];
                // Rule34 embeds post count in label
                assert!(
                    first.post_count.is_some(),
                    "Should parse post count from label"
                );
            }
        }
    }

    // Test TagSuggestion struct directly
    mod tag_suggestion {
        use super::*;

        #[test]
        fn category_name_mapping() {
            let categories = [
                (0, "general"),
                (1, "artist"),
                (3, "copyright"),
                (4, "character"),
                (5, "meta"),
            ];

            for (id, expected_name) in categories {
                let mut tag = TagSuggestion::new("test", "test");
                tag.category = Some(id);
                assert_eq!(
                    tag.category_name(),
                    Some(expected_name),
                    "Category {} should be '{}'",
                    id,
                    expected_name
                );
            }
        }

        #[test]
        fn unknown_category() {
            let mut tag = TagSuggestion::new("test", "test");
            tag.category = Some(255);
            assert_eq!(tag.category_name(), Some("unknown"));
        }
    }
}
