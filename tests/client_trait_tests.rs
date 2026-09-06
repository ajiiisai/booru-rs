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
            None => Ok(PageResult {
                posts: vec![FakePost { id: 1 }],
                next: Some(FakeContinuation(1)),
            }),
            Some(FakeContinuation(1)) => Ok(PageResult {
                posts: vec![FakePost { id: 2 }],
                next: None,
            }),
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
