//! Opt-in contract checks against the live provider APIs.
//!
//! Run with:
//! `cargo test --features live-tests --test live_contract_tests -- --ignored --nocapture`

use std::sync::OnceLock;

use booru_rs::Result;
use tokio::sync::{Mutex, MutexGuard};

static LIVE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

async fn live_test_guard() -> MutexGuard<'static, ()> {
    LIVE_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().await
}

fn endpoint(prefix: &str, default: &str) -> String {
    let name = format!("BOORU_RS_LIVE_{prefix}_ENDPOINT");
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn optional_credentials(prefix: &str) -> Option<(String, String)> {
    let key_name = format!("BOORU_RS_LIVE_{prefix}_API_KEY");
    let user_name = format!("BOORU_RS_LIVE_{prefix}_USER_ID");
    let key = std::env::var(&key_name).ok();
    let user = std::env::var(&user_name).ok();

    match (key, user) {
        (None, None) => None,
        (Some(key), Some(user)) => Some((key, user)),
        _ => panic!("set both {key_name} and {user_name} for this live test"),
    }
}

fn required_credentials(prefix: &str) -> (String, String) {
    optional_credentials(prefix).unwrap_or_else(|| {
        panic!(
            "set BOORU_RS_LIVE_{prefix}_API_KEY and BOORU_RS_LIVE_{prefix}_USER_ID for this live test"
        )
    })
}

fn assert_at_most(actual: usize, requested: u32) {
    assert!(
        actual <= requested as usize,
        "provider returned {actual} results for requested limit {requested}"
    );
}

#[tokio::test]
#[ignore = "contacts the live Danbooru API; opt in with --features live-tests -- --ignored"]
async fn danbooru_contract() -> Result<()> {
    let _guard = live_test_guard().await;
    let mut builder = booru_rs::danbooru::Client::builder()
        .endpoint(endpoint("DANBOORU", "https://danbooru.donmai.us"))?;
    if let Some((key, user)) = optional_credentials("DANBOORU") {
        builder = builder.set_credentials(key, user);
    }
    let client = builder.build()?;

    let posts = client.search().tag("cat_ears").limit(3).send().await?;
    assert!(!posts.is_empty(), "Danbooru returned no posts for cat_ears");
    assert_at_most(posts.len(), 3);

    let page = client.search().tag("cat_ears").limit(2).page().await?;
    assert_at_most(page.posts.len(), 2);
    if let Some(next) = page.next {
        let next_page = next.page().await?;
        assert_at_most(next_page.posts.len(), 2);
    }

    let suggestions = client.autocomplete("cat_", 2).await?;
    assert!(
        !suggestions.is_empty(),
        "Danbooru returned no autocomplete suggestions"
    );
    assert_at_most(suggestions.len(), 2);

    let post = posts.first().expect("nonempty search result");
    let fetched = client.post(post.id).await?;
    assert_eq!(fetched.id, post.id);
    Ok(())
}

#[tokio::test]
#[ignore = "contacts the live Gelbooru API; opt in with --features live-tests -- --ignored"]
async fn gelbooru_contract() -> Result<()> {
    let _guard = live_test_guard().await;
    let (key, user) = required_credentials("GELBOORU");
    let client = booru_rs::gelbooru::Client::builder()
        .endpoint(endpoint("GELBOORU", "https://gelbooru.com"))?
        .set_credentials(key, user)
        .build()?;

    let posts = client.search().tag("cat_ears").limit(3).send().await?;
    assert!(!posts.is_empty(), "Gelbooru returned no posts for cat_ears");
    assert_at_most(posts.len(), 3);

    let page = client.search().tag("cat_ears").limit(2).page().await?;
    assert_at_most(page.posts.len(), 2);
    if let Some(next) = page.next {
        let next_page = next.page().await?;
        assert_at_most(next_page.posts.len(), 2);
    }

    let suggestions = client.autocomplete("cat_", 2).await?;
    assert!(
        !suggestions.is_empty(),
        "Gelbooru returned no autocomplete suggestions"
    );
    assert_at_most(suggestions.len(), 2);

    let post = posts.first().expect("nonempty search result");
    let fetched = client.post(post.id).await?;
    assert_eq!(fetched.id, post.id);
    Ok(())
}

#[tokio::test]
#[ignore = "contacts the live Rule34 API; opt in with --features live-tests -- --ignored"]
async fn rule34_contract() -> Result<()> {
    let _guard = live_test_guard().await;
    let (key, user) = required_credentials("RULE34");
    let client = booru_rs::rule34::Client::builder()
        .endpoint(endpoint("RULE34", "https://api.rule34.xxx"))?
        .set_credentials(key, user)
        .build()?;

    let posts = client.search().tag("1girl").limit(3).send().await?;
    assert!(!posts.is_empty(), "Rule34 returned no posts for 1girl");
    assert_at_most(posts.len(), 3);

    let page = client.search().tag("1girl").limit(2).page().await?;
    assert_at_most(page.posts.len(), 2);
    if let Some(next) = page.next {
        let next_page = next.page().await?;
        assert_at_most(next_page.posts.len(), 2);
    }

    let suggestions = client.autocomplete("cat_", 2).await?;
    assert!(
        !suggestions.is_empty(),
        "Rule34 returned no autocomplete suggestions"
    );
    assert_at_most(suggestions.len(), 2);

    let post = posts.first().expect("nonempty search result");
    let fetched = client.post(post.id).await?;
    assert_eq!(fetched.id, post.id);
    Ok(())
}

#[tokio::test]
#[ignore = "contacts the live Safebooru API; opt in with --features live-tests -- --ignored"]
async fn safebooru_contract() -> Result<()> {
    let _guard = live_test_guard().await;
    let client = booru_rs::safebooru::Client::builder()
        .endpoint(endpoint("SAFEBOORU", "https://safebooru.org"))?
        .build()?;

    let posts = client.search().tag("landscape").limit(3).send().await?;
    assert!(
        !posts.is_empty(),
        "Safebooru returned no posts for landscape"
    );
    assert_at_most(posts.len(), 3);

    let page = client.search().tag("landscape").limit(2).page().await?;
    assert_at_most(page.posts.len(), 2);
    if let Some(next) = page.next {
        let next_page = next.page().await?;
        assert_at_most(next_page.posts.len(), 2);
    }

    let suggestions = client.autocomplete("cat_", 2).await?;
    assert!(
        !suggestions.is_empty(),
        "Safebooru returned no autocomplete suggestions"
    );
    assert_at_most(suggestions.len(), 2);

    let post = posts.first().expect("nonempty search result");
    let fetched = client.post(post.id).await?;
    assert_eq!(fetched.id, post.id);
    Ok(())
}

#[tokio::test]
#[ignore = "contacts the live Konachan API; opt in with --features live-tests -- --ignored"]
async fn konachan_contract() -> Result<()> {
    let _guard = live_test_guard().await;
    let client = booru_rs::konachan::Client::builder()
        .endpoint(endpoint("KONACHAN", "https://konachan.com"))?
        .build()?;

    let posts = client.search().tag("landscape").limit(3).send().await?;
    assert!(
        !posts.is_empty(),
        "Konachan returned no posts for landscape"
    );
    assert_at_most(posts.len(), 3);

    let page = client.search().tag("landscape").limit(2).page().await?;
    assert_at_most(page.posts.len(), 2);
    if let Some(next) = page.next {
        let next_page = next.page().await?;
        assert_at_most(next_page.posts.len(), 2);
    }

    let suggestions = client.autocomplete("cat_", 2).await?;
    assert!(
        !suggestions.is_empty(),
        "Safebooru returned no autocomplete suggestions"
    );
    assert_at_most(suggestions.len(), 2);

    let post = posts.first().expect("nonempty search result");
    let fetched = client.post(post.id).await?;
    assert_eq!(fetched.id, post.id);
    Ok(())
}