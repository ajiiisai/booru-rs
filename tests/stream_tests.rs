//! Regression tests for paginated post streams.
//!
//! Each test serves fixed pages from a local mock server and asserts the
//! exact post IDs and order yielded by `PostStream`.

use booru_rs::prelude::*;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Builds a Safebooru DAPI JSON array with one post per given ID.
fn posts_json(ids: &[u32]) -> String {
    let items: Vec<String> = ids
        .iter()
        .map(|id| {
            format!(
                concat!(
                    "{{\"id\":{id},\"score\":10,\"height\":100,\"width\":100,",
                    "\"hash\":\"hash{id}\",\"tags\":\"tag{id}\",\"image\":\"{id}.jpg\",",
                    "\"directory\":1,\"file_url\":\"https://example.com/{id}.jpg\",",
                    "\"preview_url\":\"https://example.com/p{id}.jpg\",",
                    "\"sample_url\":\"https://example.com/s{id}.jpg\",",
                    "\"source\":\"\",\"change\":1700000000,\"rating\":\"general\"}}"
                ),
                id = id
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Serves `pages[pid]` for each requested page index.
/// Callers must include a trailing empty page so the stream terminates.
async fn mock_pages(mock_server: &MockServer, pages: &[Vec<u32>]) {
    for (pid, ids) in pages.iter().enumerate() {
        Mock::given(method("GET"))
            .and(path("/index.php"))
            .and(query_param("pid", pid.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_string(posts_json(ids)))
            .mount(mock_server)
            .await;
    }
}

async fn stream_ids(pages: &[Vec<u32>]) -> Vec<u32> {
    let mock_server = MockServer::start().await;
    mock_pages(&mock_server, pages).await;

    SafebooruClient::builder()
        .with_custom_url(&mock_server.uri())
        .limit(2)
        .into_post_stream()
        .collect()
        .await
        .expect("stream should succeed")
        .iter()
        .map(|post| post.id)
        .collect()
}

#[tokio::test]
async fn post_stream_preserves_order_across_two_post_pages() {
    assert_eq!(
        stream_ids(&[vec![1, 2], vec![3, 4], vec![]]).await,
        vec![1, 2, 3, 4]
    );
}

#[tokio::test]
async fn post_stream_preserves_order_within_four_post_page() {
    assert_eq!(
        stream_ids(&[vec![1, 2, 3, 4], vec![]]).await,
        vec![1, 2, 3, 4]
    );
}

#[tokio::test]
async fn post_stream_yields_single_post_page() {
    assert_eq!(stream_ids(&[vec![7], vec![]]).await, vec![7]);
}

#[tokio::test]
async fn post_stream_stops_on_empty_first_page() {
    assert!(stream_ids(&[vec![]]).await.is_empty());
}
