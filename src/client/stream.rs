//! Shared async streams for paginating provider results.
//!
//! [`PageStream`] fetches one page at a time through the [`Client`] trait and
//! stops at the first empty page. [`PostStream`] flattens those pages into
//! individual posts. Providers alias these types instead of reimplementing
//! the polling logic:
//!
//! ```ignore
//! pub type PageStream = crate::client::stream::PageStream<Client>;
//! pub type PostStream = crate::client::stream::PostStream<Client>;
//! ```

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use super::{Client, PageResult};
use crate::error::Result;

/// Pending page fetch for [`PageStream`].
type PendingPage<C> = Pin<
    Box<
        dyn Future<Output = Result<PageResult<<C as Client>::Post, <C as Client>::Continuation>>>
            + Send,
    >,
>;

/// Stream of result pages for a provider client.
///
/// Pages stop at an empty page or when `max_pages` is reached. Build one from
/// a provider search, or directly from a client and query for generic code.
pub struct PageStream<C: Client> {
    client: C,
    query: C::Query,
    continuation: Option<C::Continuation>,
    pending: Option<PendingPage<C>>,
    fetched: u32,
    max_pages: Option<u32>,
    done: bool,
}

impl<C> PageStream<C>
where
    C: Client + Clone + Send + Sync + Unpin + 'static,
    C::Query: Clone + Send + Sync + Unpin + 'static,
    C::Continuation: Clone + Send + Sync + Unpin + 'static,
    C::Post: Send + Unpin + 'static,
{
    /// Creates a stream starting from an optional continuation.
    ///
    /// A continuation preserves the query and page position. This stream uses
    /// `client` for every request, including when it resumes from a
    /// continuation. Pass `None` to start from the first page.
    pub fn new(client: C, query: C::Query, continuation: Option<C::Continuation>) -> Self {
        Self {
            client,
            query,
            continuation,
            pending: None,
            fetched: 0,
            max_pages: None,
            done: false,
        }
    }

    /// Fetches the next page.
    pub async fn next(&mut self) -> Option<Result<PageResult<C::Post, C::Continuation>>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// Limits the number of pages fetched.
    pub fn max_pages(mut self, max: u32) -> Self {
        self.max_pages = Some(max);
        self
    }
}

impl<C> Stream for PageStream<C>
where
    C: Client + Clone + Send + Sync + Unpin + 'static,
    C::Query: Clone + Send + Sync + Unpin + 'static,
    C::Continuation: Clone + Send + Sync + Unpin + 'static,
    C::Post: Send + Unpin + 'static,
{
    type Item = Result<PageResult<C::Post, C::Continuation>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(max) = this.max_pages
            && this.fetched >= max
        {
            this.continuation = None;
            this.pending = None;
            this.done = true;
            return Poll::Ready(None);
        }
        loop {
            if this.done {
                return Poll::Ready(None);
            }
            if let Some(mut pending) = this.pending.take() {
                match pending.as_mut().poll(cx) {
                    Poll::Ready(result) => match result {
                        Ok(page) => {
                            this.fetched = this.fetched.saturating_add(1);
                            if page.posts.is_empty() {
                                this.continuation = None;
                                this.done = true;
                                return Poll::Ready(None);
                            }
                            this.done = page.next.is_none();
                            this.continuation = page.next.clone();
                            return Poll::Ready(Some(Ok(page)));
                        }
                        Err(error) => {
                            this.continuation = None;
                            this.done = true;
                            return Poll::Ready(Some(Err(error)));
                        }
                    },
                    Poll::Pending => {
                        this.pending = Some(pending);
                        return Poll::Pending;
                    }
                }
            }

            let client = this.client.clone();
            let query = this.query.clone();
            let continuation = this.continuation.take();
            this.pending = Some(Box::pin(
                async move { client.page(query, continuation).await },
            ));
        }
    }
}

/// Stream of individual posts across result pages.
///
/// Buffers each fetched page and yields its posts one at a time. Stops at an
/// empty page or when `max_posts` is reached.
pub struct PostStream<C: Client> {
    pages: PageStream<C>,
    buffer: std::vec::IntoIter<C::Post>,
    yielded: u32,
    max_posts: Option<u32>,
}

impl<C> PostStream<C>
where
    C: Client + Clone + Send + Sync + Unpin + 'static,
    C::Query: Clone + Send + Sync + Unpin + 'static,
    C::Continuation: Clone + Send + Sync + Unpin + 'static,
    C::Post: Send + Unpin + 'static,
{
    /// Creates a post stream over an existing page stream.
    pub fn new(pages: PageStream<C>) -> Self {
        Self {
            pages,
            buffer: Vec::new().into_iter(),
            yielded: 0,
            max_posts: None,
        }
    }

    /// Yields the next post.
    pub async fn next(&mut self) -> Option<Result<C::Post>> {
        std::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// Limits the total number of posts yielded.
    pub fn max_posts(mut self, max: u32) -> Self {
        self.max_posts = Some(max);
        self
    }

    /// Collects all remaining posts into a vector.
    pub async fn collect(mut self) -> Result<Vec<C::Post>> {
        let mut posts = Vec::new();
        while let Some(result) = self.next().await {
            posts.push(result?);
        }
        Ok(posts)
    }
}

impl<C> Stream for PostStream<C>
where
    C: Client + Clone + Send + Sync + Unpin + 'static,
    C::Query: Clone + Send + Sync + Unpin + 'static,
    C::Continuation: Clone + Send + Sync + Unpin + 'static,
    C::Post: Send + Unpin + 'static,
{
    type Item = Result<C::Post>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(max) = this.max_posts
            && this.yielded >= max
        {
            return Poll::Ready(None);
        }
        loop {
            if let Some(post) = this.buffer.next() {
                this.yielded = this.yielded.saturating_add(1);
                return Poll::Ready(Some(Ok(post)));
            }
            match Pin::new(&mut this.pages).poll_next(cx) {
                Poll::Ready(Some(Ok(page))) => {
                    this.buffer = page.posts.into_iter();
                }
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Some(Err(error))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
