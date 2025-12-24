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
    r#"[
        {
            "id": 12345,
            "score": 100,
            "height": 1080,
            "width": 1920,
            "hash": "abc123def456",
            "tags": "cat_ears blue_eyes",
            "image": "abc123.jpg",
            "directory": 1234,
            "file_url": "https://example.com/images/1234/abc123.jpg",
            "preview_url": "https://example.com/thumbnails/1234/thumbnail_abc123.jpg",
            "sample_url": "https://example.com/samples/1234/sample_abc123.jpg",
            "source": "https://twitter.com/artist/status/123",
            "change": 1700000000,
            "rating": "general"
        },
        {
            "id": 12346,
            "score": 50,
            "height": 720,
            "width": 1280,
            "hash": "def789ghi012",
            "tags": "landscape nature",
            "image": "def789.png",
            "directory": 1234,
            "file_url": "https://example.com/images/1234/def789.png",
            "preview_url": "https://example.com/thumbnails/1234/thumbnail_def789.png",
            "sample_url": "https://example.com/samples/1234/sample_def789.png",
            "source": "",
            "change": 1700000001,
            "rating": "safe"
        }
    ]"#
}

/// Test fixture for a single Danbooru post
fn danbooru_post_json() -> &'static str {
    r#"{
        "id": 7654321,
        "created_at": "2024-01-15T12:00:00.000Z",
        "updated_at": "2024-01-15T12:00:00.000Z",
        "uploader_id": 12345,
        "approver_id": null,
        "tag_string": "1girl solo cat_ears blue_hair",
        "tag_string_general": "1girl solo cat_ears",
        "tag_string_artist": "artist_name",
        "tag_string_copyright": "",
        "tag_string_character": "",
        "tag_string_meta": "",
        "rating": "g",
        "parent_id": null,
        "pixiv_id": null,
        "source": "https://pixiv.net/artworks/123456",
        "md5": "abcdef1234567890",
        "file_url": "https://example.com/original/abcdef.png",
        "large_file_url": "https://example.com/sample/abcdef.jpg",
        "preview_file_url": "https://example.com/preview/abcdef.jpg",
        "file_ext": "png",
        "file_size": 2500000,
        "image_width": 2048,
        "image_height": 1536,
        "score": 250,
        "up_score": 300,
        "down_score": 50,
        "fav_count": 100,
        "tag_count_general": 3,
        "tag_count_artist": 1,
        "tag_count_copyright": 0,
        "tag_count_character": 0,
        "tag_count_meta": 0,
        "last_comment_bumped_at": null,
        "last_noted_at": null,
        "has_large": true,
        "has_children": false,
        "has_visible_children": false,
        "has_active_children": false,
        "is_banned": false,
        "is_deleted": false,
        "is_flagged": false,
        "is_pending": false,
        "bit_flags": 0
    }"#
}

/// Test fixture for Danbooru posts array
fn danbooru_posts_json() -> &'static str {
    r#"[
        {
            "id": 7654321,
            "created_at": "2024-01-15T12:00:00.000Z",
            "updated_at": "2024-01-15T12:00:00.000Z",
            "uploader_id": 12345,
            "approver_id": null,
            "tag_string": "1girl solo cat_ears blue_hair",
            "tag_string_general": "1girl solo cat_ears",
            "tag_string_artist": "artist_name",
            "tag_string_copyright": "",
            "tag_string_character": "",
            "tag_string_meta": "",
            "rating": "g",
            "parent_id": null,
            "pixiv_id": null,
            "source": "https://pixiv.net/artworks/123456",
            "md5": "abcdef1234567890",
            "file_url": "https://example.com/original/abcdef.png",
            "large_file_url": "https://example.com/sample/abcdef.jpg",
            "preview_file_url": "https://example.com/preview/abcdef.jpg",
            "file_ext": "png",
            "file_size": 2500000,
            "image_width": 2048,
            "image_height": 1536,
            "score": 250,
            "up_score": 300,
            "down_score": 50,
            "fav_count": 100,
            "tag_count_general": 3,
            "tag_count_artist": 1,
            "tag_count_copyright": 0,
            "tag_count_character": 0,
            "tag_count_meta": 0,
            "last_comment_bumped_at": null,
            "last_noted_at": null,
            "has_large": true,
            "has_children": false,
            "has_visible_children": false,
            "has_active_children": false,
            "is_banned": false,
            "is_deleted": false,
            "is_flagged": false,
            "is_pending": false,
            "bit_flags": 0
        }
    ]"#
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
        let single_post = r#"[{
            "id": 12345,
            "score": 100,
            "height": 1080,
            "width": 1920,
            "hash": "abc123def456",
            "tags": "cat_ears blue_eyes",
            "image": "abc123.jpg",
            "directory": 1234,
            "file_url": "https://example.com/images/1234/abc123.jpg",
            "preview_url": "https://example.com/thumbnails/1234/thumbnail_abc123.jpg",
            "sample_url": "https://example.com/samples/1234/sample_abc123.jpg",
            "source": "",
            "change": 1700000000,
            "rating": "general"
        }]"#;

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
    use wiremock::matchers::any;

    #[tokio::test]
    async fn test_get_posts_success() {
        let mock_server = MockServer::start().await;

        // Use any() matcher since Danbooru adds query params
        Mock::given(any())
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

        Mock::given(any())
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
        r#"{
            "@attributes": {
                "limit": 1,
                "offset": 0,
                "count": 1000
            },
            "post": [
                {
                    "id": 9876543,
                    "created_at": "Mon Dec 23 12:00:00 -0600 2024",
                    "score": 150,
                    "width": 1920,
                    "height": 1080,
                    "md5": "gelbooru123abc",
                    "directory": "ge/lb",
                    "image": "gelbooru123abc.jpg",
                    "rating": "general",
                    "source": "https://twitter.com/artist/status/456",
                    "change": 1700000000,
                    "owner": "uploader_name",
                    "creator_id": 12345,
                    "parent_id": 0,
                    "sample": 1,
                    "preview_height": 150,
                    "preview_width": 200,
                    "tags": "cat_ears blue_eyes 1girl",
                    "title": "",
                    "has_notes": "false",
                    "has_comments": "false",
                    "file_url": "https://example.com/images/ge/lb/gelbooru123abc.jpg",
                    "preview_url": "https://example.com/thumbnails/ge/lb/thumbnail_gelbooru123abc.jpg",
                    "sample_url": "https://example.com/samples/ge/lb/sample_gelbooru123abc.jpg",
                    "sample_height": 720,
                    "sample_width": 1280,
                    "status": "active",
                    "post_locked": 0,
                    "has_children": "false"
                }
            ]
        }"#
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
        r#"[
            {
                "id": 15000000,
                "score": 75,
                "height": 1200,
                "width": 900,
                "hash": "rule34hash123",
                "tags": "1girl blue_hair",
                "image": "rule34hash123.png",
                "directory": 3900,
                "file_url": "https://example.com/images/3900/rule34hash123.png",
                "preview_url": "https://example.com/thumbnails/3900/thumbnail_rule34hash123.jpg",
                "sample_url": "https://example.com/samples/3900/sample_rule34hash123.jpg",
                "source": "https://pixiv.net/artworks/789",
                "change": 1700000000,
                "rating": "explicit",
                "owner": "uploader",
                "parent_id": 0,
                "sample": true,
                "sample_height": 600,
                "sample_width": 450,
                "status": "active",
                "has_notes": false,
                "comment_count": 0
            }
        ]"#
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
