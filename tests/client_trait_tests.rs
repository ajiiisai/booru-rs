use booru_rs::client::{Client, PageResult};
use booru_rs::model::Post;

#[derive(Clone)]
struct FakeQuery;

#[derive(Clone)]
struct FakeContinuation(u32);

struct FakePost {
    id: u32,
}

impl Post for FakePost {
    fn id(&self) -> u32 {
        self.id
    }

    fn width(&self) -> u32 {
        1
    }

    fn height(&self) -> Option<u32> {
        Some(1)
    }

    fn file_url(&self) -> Option<&str> {
        None
    }

    fn tags(&self) -> &str {
        ""
    }

    fn score(&self) -> Option<i64> {
        None
    }

    fn md5(&self) -> Option<&str> {
        None
    }

    fn source(&self) -> Option<&str> {
        None
    }
}

struct FakeClient;

impl Client for FakeClient {
    type Query = FakeQuery;
    type Post = FakePost;
    type Continuation = FakeContinuation;

    async fn page(
        &self,
        _query: Self::Query,
        continuation: Option<Self::Continuation>,
    ) -> booru_rs::Result<PageResult<Self::Post, Self::Continuation>> {
        match continuation {
            None => Ok(PageResult::new(
                vec![FakePost { id: 1 }],
                Some(FakeContinuation(1)),
            )),
            Some(FakeContinuation(1)) => Ok(PageResult::new(vec![FakePost { id: 2 }], None)),
            Some(FakeContinuation(_)) => unreachable!(),
        }
    }

    async fn post(&self, id: u32) -> booru_rs::Result<Self::Post> {
        Ok(FakePost { id })
    }
}

async fn collect_pages<C: Client>(client: &C, query: C::Query) -> booru_rs::Result<Vec<C::Post>> {
    let mut continuation = None;
    let mut posts = Vec::new();
    loop {
        let page = client.page(query.clone(), continuation).await?;
        posts.extend(page.posts);
        continuation = page.next;
        if continuation.is_none() {
            return Ok(posts);
        }
    }
}

#[tokio::test]
async fn external_style_client_implements_operation_interface() {
    let client = FakeClient;
    let posts = collect_pages(&client, FakeQuery).await.unwrap();
    assert_eq!(posts.iter().map(Post::id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(Client::post(&client, 42).await.unwrap().id(), 42);
}

#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru",
    feature = "konachan"
))]
mod builder_tests {
    use booru_rs::client::Builder;
    use booru_rs::ratelimit::RateLimiter;
    use booru_rs::retry::RetryConfig;
    use std::time::Duration;

    fn build_with_endpoint<B: Builder>(builder: B) -> B::Client {
        builder
            .endpoint("https://example.com")
            .unwrap()
            .build()
            .unwrap()
    }

    fn reject_bad_endpoint<B: Builder>(builder: B) {
        assert!(builder.endpoint("not a url").is_err());
    }

    fn apply_policy<B: Builder>(builder: B) -> B::Client {
        builder
            .retry_config(RetryConfig::new(1).with_initial_delay(Duration::ZERO))
            .unwrap()
            .rate_limiter(RateLimiter::new(10, Duration::from_secs(1)).unwrap())
            .http_client(reqwest::Client::new())
            .build()
            .unwrap()
    }

    #[test]
    #[cfg(feature = "danbooru")]
    fn danbooru_builder_implements_shared_trait() {
        build_with_endpoint(booru_rs::danbooru::Client::builder());
        reject_bad_endpoint(booru_rs::danbooru::Client::builder());
        apply_policy(booru_rs::danbooru::Client::builder());
    }

    #[test]
    #[cfg(feature = "gelbooru")]
    fn gelbooru_builder_implements_shared_trait() {
        build_with_endpoint(booru_rs::gelbooru::Client::builder());
        reject_bad_endpoint(booru_rs::gelbooru::Client::builder());
        apply_policy(booru_rs::gelbooru::Client::builder());
    }

    #[test]
    #[cfg(feature = "rule34")]
    fn rule34_builder_implements_shared_trait() {
        build_with_endpoint(booru_rs::rule34::Client::builder());
        reject_bad_endpoint(booru_rs::rule34::Client::builder());
        apply_policy(booru_rs::rule34::Client::builder());
    }

    #[test]
    #[cfg(feature = "safebooru")]
    fn safebooru_builder_implements_shared_trait() {
        build_with_endpoint(booru_rs::safebooru::Client::builder());
        reject_bad_endpoint(booru_rs::safebooru::Client::builder());
        apply_policy(booru_rs::safebooru::Client::builder());
    }

    #[test]
    #[cfg(feature = "konachan")]
    fn konachan_builder_implements_shared_trait() {
        build_with_endpoint(booru_rs::konachan::Client::builder());
        reject_bad_endpoint(booru_rs::konachan::Client::builder());
        apply_policy(booru_rs::konachan::Client::builder());
    }
}

/// Proves every provider exposes the same query and search surface.
///
/// Copy one block per new provider following `docs/new-provider.md`. A
/// missing method fails to compile, which is the point. No block sends a
/// request: futures are built and dropped without polling.
#[cfg(any(
    feature = "danbooru",
    feature = "gelbooru",
    feature = "rule34",
    feature = "safebooru",
    feature = "konachan"
))]
mod surface_tests {
    use booru_rs::client::generic::Sort;

    #[test]
    #[cfg(feature = "danbooru")]
    fn danbooru_matches_provider_surface() {
        use booru_rs::danbooru::{Client, DanbooruRating};

        let client = Client::builder().build().unwrap();
        let search = client
            .search()
            .tag("cat")
            .tags(["pink_hair"])
            .raw_query("artist:foo")
            .raw_queries(["score:>10"])
            .rating(DanbooruRating::General)
            .sort(Sort::Score)
            .limit(10)
            .blacklist_tag("spoiler")
            .blacklist_tags(["gore"])
            .exclude_rating(DanbooruRating::Explicit);
        // Danbooru allows 2 tags including negations, so validate the
        // chain shape on a query inside the limit instead.
        let query = booru_rs::danbooru::Query::new()
            .tag("cat")
            .rating(DanbooruRating::General)
            .sort(Sort::Score)
            .limit(10);
        assert!(query.validate().is_ok());
        let _ = search.clone().pages().max_pages(1);
        let _ = search.clone().posts().max_posts(1);
        drop(search.posts().max_posts(1).collect());
    }

    #[test]
    #[cfg(feature = "gelbooru")]
    fn gelbooru_matches_provider_surface() {
        use booru_rs::gelbooru::{Client, GelbooruRating};

        let client = Client::builder().build().unwrap();
        let search = client
            .search()
            .tag("cat")
            .tags(["smile", "pink_hair"])
            .raw_query("artist:foo")
            .raw_queries(["score:>10"])
            .rating(GelbooruRating::General)
            .sort(Sort::Score)
            .limit(10)
            .blacklist_tag("spoiler")
            .blacklist_tags(["gore"])
            .exclude_rating(GelbooruRating::Explicit);
        assert!(search.query().validate().is_ok());
        let _ = search.clone().pages().max_pages(1);
        let _ = search.clone().posts().max_posts(1);
        drop(search.posts().max_posts(1).collect());
    }

    #[test]
    #[cfg(feature = "safebooru")]
    fn safebooru_matches_provider_surface() {
        use booru_rs::safebooru::{Client, SafebooruRating};

        let client = Client::builder().build().unwrap();
        let search = client
            .search()
            .tag("cat")
            .tags(["smile", "pink_hair"])
            .raw_query("artist:foo")
            .raw_queries(["score:>10"])
            .rating(SafebooruRating::Safe)
            .sort(Sort::Score)
            .limit(10)
            .blacklist_tag("spoiler")
            .blacklist_tags(["gore"])
            .exclude_rating(SafebooruRating::Explicit);
        assert!(search.query().validate().is_ok());
        let _ = search.clone().pages().max_pages(1);
        let _ = search.clone().posts().max_posts(1);
        drop(search.posts().max_posts(1).collect());
    }

    #[test]
    #[cfg(feature = "rule34")]
    fn rule34_matches_provider_surface() {
        use booru_rs::rule34::{Client, Rule34Rating};

        let client = Client::builder().build().unwrap();
        let search = client
            .search()
            .tag("1girl")
            .tags(["smile", "pink_hair"])
            .raw_query("artist:foo")
            .raw_queries(["score:>10"])
            .rating(Rule34Rating::General)
            .sort(Sort::Score)
            .limit(10)
            .blacklist_tag("spoiler")
            .blacklist_tags(["gore"])
            .exclude_rating(Rule34Rating::Explicit);
        assert!(search.query().validate().is_ok());
        let _ = search.clone().pages().max_pages(1);
        let _ = search.clone().posts().max_posts(1);
        drop(search.posts().max_posts(1).collect());
    }

    #[test]
    #[cfg(feature = "konachan")]
    fn konachan_matches_provider_surface() {
        use booru_rs::konachan::{Client, KonachanRating};

        let client = Client::builder().build().unwrap();
        let search = client
            .search()
            .tag("cat")
            .tags(["smile", "pink_hair"])
            .raw_query("artist:foo")
            .raw_queries(["score:>10"])
            .rating(KonachanRating::Safe)
            .sort(Sort::Score)
            .limit(10)
            .blacklist_tag("spoiler")
            .blacklist_tags(["gore"])
            .exclude_rating(KonachanRating::Explicit);
        assert!(search.query().validate().is_ok());
        let _ = search.clone().pages().max_pages(1);
        let _ = search.clone().posts().max_posts(1);
        drop(search.posts().max_posts(1).collect());
    }
}
