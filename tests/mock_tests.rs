//! Mock server tests for booru clients.
//!
//! These tests use wiremock to create a local mock server, allowing us to test
//! client behavior without hitting real APIs. This is useful for:
//! - Testing error handling
//! - Testing edge cases
//! - Running fast, reliable tests in CI
//! - Testing without API credentials

use booru_rs::error::BooruError;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test fixture for Safebooru posts
fn safebooru_posts_json() -> &'static str {
    include_str!("fixtures/safebooru/posts.json")
}

mod mock_post_trait {
    use super::*;
    use booru_rs::model::Post;
    use booru_rs::safebooru::Client;

    #[tokio::test]
    async fn test_post_trait_methods() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(safebooru_posts_json()))
            .mount(&mock_server)
            .await;

        let client = Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .build()
            .unwrap();

        let posts = client.search().send().await.unwrap();
        let post = &posts[0];

        assert_eq!(post.id(), 12345);
        assert_eq!(post.width(), 1920);
        assert_eq!(post.height(), Some(1080));
        assert_eq!(
            post.file_url(),
            Some("https://example.com/images/1234/abc123.jpg")
        );
        assert_eq!(post.tags(), "cat_ears blue_eyes");
        assert_eq!(
            post.tags_iter().collect::<Vec<_>>(),
            ["cat_ears", "blue_eyes"]
        );
        assert_eq!(post.score(), Some(100));
        assert_eq!(post.md5(), Some("abc123def456"));
        assert_eq!(post.source(), Some("https://twitter.com/artist/status/123"));
    }

    #[tokio::test]
    async fn test_post_trait_empty_source() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(safebooru_posts_json()))
            .mount(&mock_server)
            .await;

        let client = Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .build()
            .unwrap();

        let posts = client.search().send().await.unwrap();
        let post = &posts[1]; // Second post has empty source

        assert_eq!(post.source(), None);
    }
}

mod mock_autocomplete {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "danbooru")]
    async fn zero_limit_skips_danbooru_request() {
        let mock_server = MockServer::start().await;
        let client = booru_rs::danbooru::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .build()
            .unwrap();

        assert!(client.autocomplete("cat_", 0).await.unwrap().is_empty());
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "gelbooru")]
    async fn zero_limit_skips_gelbooru_request() {
        let mock_server = MockServer::start().await;
        let client = booru_rs::gelbooru::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build()
            .unwrap();

        assert!(client.autocomplete("cat_", 0).await.unwrap().is_empty());
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "rule34")]
    async fn zero_limit_skips_rule34_request() {
        let mock_server = MockServer::start().await;
        let client = booru_rs::rule34::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build()
            .unwrap();

        assert!(client.autocomplete("cat_", 0).await.unwrap().is_empty());
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "safebooru")]
    async fn zero_limit_skips_safebooru_request() {
        let mock_server = MockServer::start().await;
        let client = booru_rs::safebooru::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .build()
            .unwrap();

        assert!(client.autocomplete("cat_", 0).await.unwrap().is_empty());
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_danbooru_uses_instance_endpoint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/autocomplete.json"))
            .and(query_param("search[query]", "cat_"))
            .and(query_param("limit", "3"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"[{"value":"cat_ears","label":"cat_ears (177448)","category":0,"post_count":177448}]"#,
            ))
            .mount(&mock_server)
            .await;

        let client = booru_rs::danbooru::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .build()
            .unwrap();

        let suggestions = client
            .autocomplete("cat_", 3)
            .await
            .expect("complete must succeed");

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].name, "cat_ears");
        assert_eq!(suggestions[0].post_count, Some(177448));
        assert_eq!(suggestions[0].category, Some(0));
    }

    #[tokio::test]
    #[cfg(feature = "gelbooru")]
    async fn test_gelbooru_uses_instance_endpoint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("page", "autocomplete2"))
            .and(query_param("term", "cat_"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"[{"value":"cat_ears","label":"cat ears","post_count":"430787","category":"tag"},{"value":"cat_tail","label":"cat tail","post_count":"243121","category":"tag"},{"value":"cat_girl","label":"cat girl","post_count":"167612","category":"tag"}]"#),
            )
            .mount(&mock_server)
            .await;

        let client = booru_rs::gelbooru::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build()
            .unwrap();

        let suggestions = client
            .autocomplete("cat_", 2)
            .await
            .expect("complete must succeed");

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].name, "cat_ears");
        assert_eq!(suggestions[0].post_count, Some(430787));
        assert_eq!(suggestions[0].category, Some(0));
    }

    #[tokio::test]
    async fn test_safebooru_uses_instance_endpoint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/autocomplete.php"))
            .and(query_param("q", "land"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"[{"value":"landscape","label":"landscape (123)"}]"#),
            )
            .mount(&mock_server)
            .await;

        let client = booru_rs::safebooru::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .build()
            .unwrap();

        let suggestions = client
            .autocomplete("land", 5)
            .await
            .expect("complete must succeed");

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].name, "landscape");
        assert_eq!(suggestions[0].post_count, Some(123));
    }

    #[tokio::test]
    #[cfg(feature = "rule34")]
    async fn test_rule34_honors_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/autocomplete.php"))
            .and(query_param("q", "cat_"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"[{"value":"cat_ears","label":"cat_ears (1)"},{"value":"cat_girl","label":"cat_girl (2)"},{"value":"cat_tail","label":"cat_tail (3)"}]"#,
            ))
            .mount(&mock_server)
            .await;

        let client = booru_rs::rule34::Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build()
            .unwrap();

        let suggestions = client
            .autocomplete("cat_", 2)
            .await
            .expect("complete must succeed");

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].name, "cat_ears");
        assert_eq!(suggestions[1].name, "cat_girl");
    }
}

mod endpoint_config {
    use super::*;
    use booru_rs::safebooru::Client;

    #[tokio::test]
    async fn test_trailing_slash_still_routes() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("id", "12345"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(include_str!("fixtures/safebooru/post.json")),
            )
            .mount(&mock_server)
            .await;

        let post = Client::builder()
            .endpoint(format!("{}/", mock_server.uri()))
            .unwrap()
            .build()
            .unwrap()
            .post(12345)
            .await
            .expect("lookup must succeed");

        assert_eq!(post.id, 12345);
    }

    #[test]
    fn test_blank_endpoint_is_invalid() {
        let result = Client::builder().endpoint("   ");

        assert!(matches!(result.unwrap_err(), BooruError::InvalidUrl(_)));
    }

    #[test]
    fn test_unparsable_endpoint_is_invalid() {
        let result = Client::builder().endpoint("not a url");

        assert!(matches!(result.unwrap_err(), BooruError::InvalidUrl(_)));
    }

    #[test]
    fn test_non_http_endpoint_is_invalid() {
        let result = Client::builder().endpoint("ftp://example.com/x");

        assert!(matches!(result.unwrap_err(), BooruError::InvalidUrl(_)));
    }
}

mod credential_redaction {
    #[test]
    fn client_debug_output_omits_credentials() {
        let secret = "debug-secret";
        let user = "debug-user";

        let danbooru_builder = booru_rs::danbooru::Client::builder().set_credentials(secret, user);
        let gelbooru_builder = booru_rs::gelbooru::Client::builder().set_credentials(secret, user);
        let rule34_builder = booru_rs::rule34::Client::builder().set_credentials(secret, user);

        for debug in [
            format!("{danbooru_builder:?}"),
            format!("{gelbooru_builder:?}"),
            format!("{rule34_builder:?}"),
            format!("{:?}", danbooru_builder.build().unwrap()),
            format!("{:?}", gelbooru_builder.build().unwrap()),
            format!("{:?}", rule34_builder.build().unwrap()),
        ] {
            assert!(!debug.contains(secret));
            assert!(!debug.contains(user));
        }
    }

    #[tokio::test]
    async fn request_errors_omit_credentials() {
        let secret = "request-secret";
        let user = "request-user";
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let client = booru_rs::danbooru::Client::builder()
            .endpoint(endpoint)
            .unwrap()
            .set_credentials(secret, user)
            .build()
            .unwrap();

        let error = client.post(1).await.unwrap_err();
        let diagnostics = format!("{error:?} {error}");
        assert!(!diagnostics.contains(secret));
        assert!(!diagnostics.contains(user));
    }
}
