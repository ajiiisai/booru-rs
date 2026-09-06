//! Integration tests for tag autocomplete functionality.

#[cfg(test)]
mod autocomplete {
    use booru_rs::autocomplete::{Autocomplete, TagSuggestion};

    #[cfg(feature = "danbooru")]
    mod danbooru {
        use booru_rs::danbooru::Client;

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_returns_suggestions() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_", 10)
                .await;

            assert!(suggestions.is_ok(), "Autocomplete request failed");
            let suggestions = suggestions.unwrap();
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_respects_limit() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("a", 5)
                .await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
            assert!(suggestions.len() <= 5, "Should respect limit parameter");
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_has_tag_names() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_ears", 5)
                .await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
            if !suggestions.is_empty() {
                let first = &suggestions[0];
                assert!(!first.name.is_empty(), "Tag name should not be empty");
                assert!(!first.label.is_empty(), "Label should not be empty");
            }
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_returns_post_counts() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_ears", 5)
                .await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
            // Danbooru should return post counts
            if !suggestions.is_empty() {
                assert!(
                    suggestions[0].post_count.is_some(),
                    "Danbooru should provide post counts"
                );
            }
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_returns_categories() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_ears", 5)
                .await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
            // Danbooru should return category info
            if !suggestions.is_empty() {
                assert!(
                    suggestions[0].category.is_some(),
                    "Danbooru should provide category info"
                );
            }
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_empty_query() {
            // Empty query should still work (returns popular tags or empty)
            let suggestions = Client::builder().build().unwrap().autocomplete("", 5).await;
            assert!(suggestions.is_ok());
        }
    }

    #[cfg(feature = "safebooru")]
    mod safebooru {
        use booru_rs::safebooru::Client;

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_returns_suggestions() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_", 10)
                .await;

            assert!(suggestions.is_ok(), "Autocomplete request failed");
            let suggestions = suggestions.unwrap();
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_respects_limit() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("a", 5)
                .await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
            assert!(suggestions.len() <= 5, "Should respect limit parameter");
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_parses_post_count_from_label() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_ears", 5)
                .await;

            assert!(suggestions.is_ok());
            let suggestions = suggestions.unwrap();
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
        use booru_rs::gelbooru::Client;

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_returns_suggestions() {
            // Gelbooru autocomplete may work without auth
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("cat_", 10)
                .await;

            // This might fail due to auth requirements, which is expected
            if let Ok(suggestions) = suggestions {
                assert!(
                    !suggestions.is_empty(),
                    "Should return at least one suggestion"
                );
            }
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_respects_limit() {
            let suggestions = Client::builder()
                .build()
                .unwrap()
                .autocomplete("a", 5)
                .await;

            if let Ok(suggestions) = suggestions {
                assert!(suggestions.len() <= 5, "Should respect limit parameter");
            }
        }
    }

    #[cfg(feature = "rule34")]
    mod rule34 {
        use super::*;
        use booru_rs::client::Client;
        use booru_rs::rule34::Rule34Client;

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_returns_suggestions() {
            let suggestions = Rule34Client::builder()
                .build()
                .autocomplete("cat_", 10)
                .await;

            assert!(suggestions.is_ok(), "Rule34 autocomplete should work");
            let suggestions = suggestions.unwrap();
            assert!(
                !suggestions.is_empty(),
                "Should return at least one suggestion"
            );
        }

        #[tokio::test]
        #[ignore = "contacts a live booru service; run explicitly with --ignored"]
        async fn autocomplete_parses_post_count() {
            let suggestions = Rule34Client::builder()
                .build()
                .autocomplete("cat_ears", 5)
                .await;

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
