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
            .and(query_param("pid", "0"))
            .and(query_param("limit", "10"))
            .and(query_param("tags", "cat_ears"))
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string(gelbooru_response_json()))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .endpoint(mock_server.uri())
            .unwrap()
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
            .and(query_param("api_key", "bad_key"))
            .and(query_param("user_id", "bad_user"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .endpoint(mock_server.uri())
            .unwrap()
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
            .and(query_param("tags", "nonexistent_tag_xyz"))
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string(empty_response))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .tag("nonexistent_tag_xyz")
            .unwrap()
            .build();

        let posts = client.get().await;

        assert!(posts.is_ok());
        assert!(posts.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_post_not_found_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("id", "99999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let client = GelbooruClient::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build();

        let result = client.get_by_id(99999).await;

        assert!(matches!(
            result.unwrap_err(),
            BooruError::PostNotFound(99999)
        ));
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
            .and(query_param("pid", "0"))
            .and(query_param("limit", "10"))
            .and(query_param("tags", "1girl"))
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string(rule34_posts_json()))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
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
        let error_response =
            r#""Missing authentication. Go to api.rule34.xxx for more information""#;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("api_key", "bad_key"))
            .and(query_param("user_id", "bad_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string(error_response))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
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
            .and(query_param("tags", "nonexistent_tag_xyz"))
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
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
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string(rule34_posts_json()))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
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

    #[tokio::test]
    async fn test_posts_containing_auth_text_decode() {
        let mock_server = MockServer::start().await;

        let body = r#"[{"id":15000001,"score":1,"width":100,"height":100,"file_url":"https://example.com/a.png","preview_url":"https://example.com/p.png","sample_url":"https://example.com/s.png","tags":"1girl","rating":"safe","source":"artwork about Missing authentication","hash":"abc"}]"#;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("tags", "1girl"))
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .tag("1girl")
            .unwrap()
            .build();

        let posts = client.get().await.expect("valid posts must decode");

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].id, 15000001);
    }

    #[tokio::test]
    async fn test_malformed_body_reports_parse_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("api_key", "test_key"))
            .and(query_param("user_id", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build();

        let result = client.get().await;

        let error = result.unwrap_err();
        assert!(matches!(error, BooruError::Parse(_)));
        assert!(error.is_parse_error());
    }
}

mod mock_autocomplete {
    use super::*;
    use booru_rs::autocomplete::Autocomplete;
    use booru_rs::prelude::*;

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

        let client = GelbooruClient::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build();

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

        let client = Rule34Client::builder()
            .endpoint(mock_server.uri())
            .unwrap()
            .set_credentials("test_key", "test_user")
            .build();

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
