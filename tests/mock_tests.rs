//! Mock server tests for booru clients.
//!
//! These tests use wiremock to create a local mock server, allowing us to test
//! client behavior without hitting real APIs. This is useful for:
//! - Testing error handling
//! - Testing edge cases
//! - Running fast, reliable tests in CI
//! - Testing without API credentials

use booru_rs::client::Client;
use booru_rs::error::BooruError;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test fixture for Safebooru posts
fn safebooru_posts_json() -> &'static str {
    include_str!("fixtures/safebooru/posts.json")
}

/// Test fixture for a single Danbooru post
fn danbooru_post_json() -> &'static str {
    include_str!("fixtures/danbooru/post.json")
}

/// Test fixture for Danbooru posts array
fn danbooru_posts_json() -> &'static str {
    include_str!("fixtures/danbooru/posts.json")
}

mod mock_safebooru {
    use super::*;
    use booru_rs::prelude::*;

    #[tokio::test]
    async fn test_get_posts_success() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Set up the mock response
        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("page", "dapi"))
            .and(query_param("s", "post"))
            .and(query_param("q", "index"))
            .and(query_param("json", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(safebooru_posts_json()))
            .mount(&mock_server)
            .await;

        // Create client pointing to mock server
        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .tag("cat_ears")
            .unwrap()
            .limit(10)
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        assert!(posts.is_ok());
        let posts = posts.unwrap();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].id, 12345);
        assert_eq!(posts[0].hash, "abc123def456");
        assert_eq!(posts[1].id, 12346);
    }

    #[tokio::test]
    async fn test_get_post_by_id_success() {
        let mock_server = MockServer::start().await;

        // Single post wrapped in array for Safebooru
        let single_post = include_str!("fixtures/safebooru/post.json");

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("id", "12345"))
            .respond_with(ResponseTemplate::new(200).set_body_string(single_post))
            .mount(&mock_server)
            .await;

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let post = client.get_by_id(12345).await;

        assert!(post.is_ok());
        let post = post.unwrap();
        assert_eq!(post.id, 12345);
        assert_eq!(post.width, 1920);
        assert_eq!(post.height, 1080);
    }

    #[tokio::test]
    async fn test_get_post_not_found() {
        let mock_server = MockServer::start().await;

        // Empty array means post not found
        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("id", "99999"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&mock_server)
            .await;

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let result = client.get_by_id(99999).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            BooruError::PostNotFound(99999)
        ));
    }

    #[tokio::test]
    async fn test_empty_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&mock_server)
            .await;

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .tag("nonexistent_tag_xyz")
            .unwrap()
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let result = client.get().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let result = client.get().await;

        // Invalid JSON causes a Request error (reqwest's json parsing)
        assert!(result.is_err());
    }
}

mod mock_danbooru {
    use super::*;
    use booru_rs::prelude::*;

    #[tokio::test]
    async fn test_get_posts_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/posts.json"))
            .and(query_param("limit", "10"))
            .and(query_param("page", "0"))
            .and(query_param("tags", "cat_ears"))
            .and(header("User-Agent", "booru-rs/0.3.0"))
            .respond_with(ResponseTemplate::new(200).set_body_string(danbooru_posts_json()))
            .mount(&mock_server)
            .await;

        let client = DanbooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .tag("cat_ears")
            .unwrap()
            .limit(10)
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok(), "Expected Ok, got: {:?}", posts);
        let posts = posts.unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].id, 7654321);
        assert_eq!(posts[0].score, 250);
    }

    #[tokio::test]
    async fn test_get_post_by_id_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/posts/7654321.json"))
            .and(header("User-Agent", "booru-rs/0.3.0"))
            .respond_with(ResponseTemplate::new(200).set_body_string(danbooru_post_json()))
            .mount(&mock_server)
            .await;

        let client = DanbooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let post = client.get_by_id(7654321).await;

        assert!(post.is_ok(), "Expected Ok, got: {:?}", post);
        let post = post.unwrap();
        assert_eq!(post.id, 7654321);
        assert_eq!(post.image_width, 2048);
    }

    #[tokio::test]
    async fn test_tag_limit_exceeded() {
        // Danbooru has a 2-tag limit
        let result = DanbooruClient::builder().tags(["tag1", "tag2", "tag3"]);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            BooruError::TagLimitExceeded {
                client: "DanbooruClient",
                max: 2,
                actual: 3
            }
        ));
    }

    #[tokio::test]
    async fn test_post_not_found_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/posts/99999.json"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_string(r#"{"success":false,"message":"not found"}"#),
            )
            .mount(&mock_server)
            .await;

        let client = DanbooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let result = client.get_by_id(99999).await;

        assert!(result.is_err());
    }
}

mod mock_post_trait {
    use super::*;
    use booru_rs::model::Post;
    use booru_rs::prelude::*;

    #[tokio::test]
    async fn test_post_trait_methods() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(safebooru_posts_json()))
            .mount(&mock_server)
            .await;

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let posts = client.get().await.unwrap();
        let post = &posts[0];

        // Test Post trait methods
        assert_eq!(post.id(), 12345);
        assert_eq!(post.width(), 1920);
        assert_eq!(post.height(), 1080);
        assert_eq!(
            post.file_url(),
            Some("https://example.com/images/1234/abc123.jpg")
        );
        assert_eq!(post.tags(), "cat_ears blue_eyes");
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

        let client = SafebooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .build();

        let posts = client.get().await.unwrap();
        let post = &posts[1]; // Second post has empty source

        // Empty source should return None
        assert_eq!(post.source(), None);
    }
}

#[cfg(feature = "gelbooru")]
mod mock_gelbooru {
    use super::*;
    use booru_rs::prelude::*;

    /// Test fixture for Gelbooru posts (wrapped in @attributes + post array)
    fn gelbooru_response_json() -> &'static str {
        include_str!("fixtures/gelbooru/posts.json")
    }

    #[tokio::test]
    async fn test_get_posts_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("page", "dapi"))
            .and(query_param("s", "post"))
            .and(query_param("q", "index"))
            .and(query_param("json", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(gelbooru_response_json()))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("test_key", "test_user")
            .tag("cat_ears")
            .unwrap()
            .limit(10)
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        let posts = posts.unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].id, 9876543);
        assert_eq!(posts[0].md5, "gelbooru123abc");
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let mock_server = MockServer::start().await;

        // Gelbooru returns empty response for unauthorized
        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("bad_key", "bad_user")
            .build();

        let result = client.get().await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            BooruError::Unauthorized { .. }
        ));
    }

    #[tokio::test]
    async fn test_empty_response() {
        let mock_server = MockServer::start().await;

        let empty_response =
            r#"{"@attributes": {"limit": 0, "offset": 0, "count": 0}, "post": []}"#;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(empty_response))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("test_key", "test_user")
            .tag("nonexistent_tag_xyz")
            .unwrap()
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().is_empty());
    }
}

#[cfg(feature = "rule34")]
mod mock_rule34 {
    use super::*;
    use booru_rs::prelude::*;

    /// Test fixture for Rule34 posts (same format as Safebooru)
    fn rule34_posts_json() -> &'static str {
        include_str!("fixtures/rule34/posts.json")
    }

    #[tokio::test]
    async fn test_get_posts_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("page", "dapi"))
            .and(query_param("s", "post"))
            .and(query_param("q", "index"))
            .and(query_param("json", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(rule34_posts_json()))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("test_key", "test_user")
            .tag("1girl")
            .unwrap()
            .limit(10)
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        let posts = posts.unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].id, 15000000);
        assert_eq!(posts[0].hash, "rule34hash123");
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let mock_server = MockServer::start().await;

        // Rule34 returns "Missing authentication" text for unauthorized
        let error_response = "Missing authentication";

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(error_response))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("bad_key", "bad_user")
            .build();

        let result = client.get().await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            BooruError::Unauthorized { .. }
        ));
    }

    #[tokio::test]
    async fn test_empty_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("test_key", "test_user")
            .tag("nonexistent_tag_xyz")
            .unwrap()
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_post_trait_methods() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(rule34_posts_json()))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .with_custom_url(&mock_server.uri())
            .set_credentials("test_key", "test_user")
            .build();

        let posts = client.get().await.unwrap();
        let post = &posts[0];

        use booru_rs::model::Post;
        assert_eq!(post.id(), 15000000);
        assert_eq!(post.width(), 900);
        assert_eq!(post.height(), 1200);
        assert_eq!(
            post.file_url(),
            Some("https://example.com/images/3900/rule34hash123.png")
        );
        assert_eq!(post.tags(), "1girl blue_hair");
        assert_eq!(post.score(), Some(75));
        assert_eq!(post.md5(), Some("rule34hash123"));
        assert_eq!(post.source(), Some("https://pixiv.net/artworks/789"));
    }
}
