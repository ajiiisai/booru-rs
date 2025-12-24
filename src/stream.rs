//! Async stream utilities for paginated results.
//!
//! This module provides utilities for iterating through paginated
//! booru results using async streams.

use crate::client::{Client, ClientBuilder};
use crate::error::Result;

/// An async stream that yields pages of posts.
///
/// Created by [`ClientBuilder::into_page_stream`] or [`ClientBuilder::into_post_stream`].
///
/// # Example
///
/// ```no_run
/// use booru_rs::prelude::*;
/// use booru_rs::stream::PageStream;
///
/// # async fn example() -> Result<()> {
/// let mut stream = SafebooruClient::builder()
///     .tag("landscape")?
///     .limit(100)
///     .into_page_stream();
///
/// // Manually poll pages
/// while let Some(page_result) = stream.next().await {
///     let posts = page_result?;
///     if posts.is_empty() {
///         break;
///     }
///     println!("Got {} posts", posts.len());
/// }
/// # Ok(())
/// # }
/// ```
pub struct PageStream<T: Client> {
    builder: ClientBuilder<T>,
    current_page: u32,
    exhausted: bool,
    max_pages: Option<u32>,
}

impl<T: Client> PageStream<T> {
    /// Creates a new page stream from a client builder.
    pub fn new(builder: ClientBuilder<T>) -> Self {
        let current_page = builder.page;
        Self {
            builder,
            current_page,
            exhausted: false,
            max_pages: None,
        }
    }

    /// Sets the maximum number of pages to fetch.
    #[must_use]
    pub fn max_pages(mut self, max: u32) -> Self {
        self.max_pages = Some(max);
        self
    }

    /// Returns the current page number.
    pub fn current_page(&self) -> u32 {
        self.current_page
    }

    /// Fetches the next page of results.
    ///
    /// Returns `None` when there are no more pages or the max page limit is reached.
    pub async fn next(&mut self) -> Option<Result<Vec<T::Post>>> {
        if self.exhausted {
            return None;
        }

        // Check max pages limit
        if let Some(max) = self.max_pages {
            let pages_fetched = self.current_page.saturating_sub(self.builder.page);
            if pages_fetched >= max {
                self.exhausted = true;
                return None;
            }
        }

        // Build client for current page
        let mut page_builder = self.builder.clone();
        page_builder.page = self.current_page;
        let client = page_builder.build();

        match client.get().await {
            Ok(posts) => {
                if posts.is_empty() {
                    self.exhausted = true;
                    return Some(Ok(posts));
                }
                self.current_page += 1;
                Some(Ok(posts))
            }
            Err(e) => {
                self.exhausted = true;
                Some(Err(e))
            }
        }
    }
}

/// An async stream that yields individual posts across pages.
///
/// This stream automatically handles pagination, fetching new pages
/// as needed while yielding posts one at a time.
///
/// # Example
///
/// ```no_run
/// use booru_rs::prelude::*;
///
/// # async fn example() -> Result<()> {
/// let mut stream = SafebooruClient::builder()
///     .tag("landscape")?
///     .limit(100)
///     .into_post_stream()
///     .max_posts(500); // Limit to 500 posts total
///
/// let mut count = 0;
/// while let Some(post_result) = stream.next().await {
///     let post = post_result?;
///     println!("Post #{}", post.id);
///     count += 1;
/// }
/// println!("Fetched {} posts", count);
/// # Ok(())
/// # }
/// ```
pub struct PostStream<T: Client> {
    page_stream: PageStream<T>,
    buffer: Vec<T::Post>,
    buffer_index: usize,
    posts_yielded: u32,
    max_posts: Option<u32>,
}

impl<T: Client> PostStream<T> {
    /// Creates a new post stream from a client builder.
    pub fn new(builder: ClientBuilder<T>) -> Self {
        Self {
            page_stream: PageStream::new(builder),
            buffer: Vec::new(),
            buffer_index: 0,
            posts_yielded: 0,
            max_posts: None,
        }
    }

    /// Sets the maximum number of posts to yield.
    #[must_use]
    pub fn max_posts(mut self, max: u32) -> Self {
        self.max_posts = Some(max);
        self
    }

    /// Sets the maximum number of pages to fetch.
    #[must_use]
    pub fn max_pages(mut self, max: u32) -> Self {
        self.page_stream = self.page_stream.max_pages(max);
        self
    }

    /// Returns the number of posts yielded so far.
    pub fn posts_yielded(&self) -> u32 {
        self.posts_yielded
    }

    /// Returns the current page number.
    pub fn current_page(&self) -> u32 {
        self.page_stream.current_page()
    }

    /// Fetches the next post.
    ///
    /// Returns `None` when there are no more posts.
    pub async fn next(&mut self) -> Option<Result<T::Post>> {
        // Check max posts limit
        if let Some(max) = self.max_posts
            && self.posts_yielded >= max
        {
            return None;
        }

        // If we have posts in the buffer, return the next one
        if self.buffer_index < self.buffer.len() {
            let post = self.buffer.swap_remove(self.buffer_index);
            // Note: swap_remove changes order but we're consuming, so OK
            self.buffer_index = 0; // Reset since swap_remove moves last to current
            self.posts_yielded += 1;
            return Some(Ok(post));
        }

        // Need to fetch more posts
        match self.page_stream.next().await? {
            Ok(posts) => {
                if posts.is_empty() {
                    return None;
                }
                self.buffer = posts;
                self.buffer_index = 1; // Will return index 0
                self.posts_yielded += 1;

                // Pop the first post
                if self.buffer.is_empty() {
                    None
                } else {
                    Some(Ok(self.buffer.swap_remove(0)))
                }
            }
            Err(e) => Some(Err(e)),
        }
    }

    /// Collects all remaining posts into a vector.
    ///
    /// This is useful when you want all posts at once. Respects `max_posts` if set.
    ///
    /// # Errors
    ///
    /// Returns the first error encountered during pagination.
    pub async fn collect(mut self) -> Result<Vec<T::Post>> {
        let mut all_posts = Vec::new();

        while let Some(result) = self.next().await {
            all_posts.push(result?);
        }

        Ok(all_posts)
    }
}

// Extend ClientBuilder with stream methods
impl<T: Client> ClientBuilder<T> {
    /// Creates an async stream that yields pages of posts.
    ///
    /// Each call to `next()` fetches and returns a full page of posts.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::prelude::*;
    ///
    /// # async fn example() -> Result<()> {
    /// let mut stream = SafebooruClient::builder()
    ///     .tag("landscape")?
    ///     .limit(100)
    ///     .into_page_stream();
    ///
    /// while let Some(page_result) = stream.next().await {
    ///     let posts = page_result?;
    ///     if posts.is_empty() { break; }
    ///     println!("Page with {} posts", posts.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn into_page_stream(self) -> PageStream<T> {
        PageStream::new(self)
    }

    /// Creates an async stream that yields individual posts.
    ///
    /// Automatically handles pagination, fetching new pages as needed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use booru_rs::prelude::*;
    ///
    /// # async fn example() -> Result<()> {
    /// let mut stream = SafebooruClient::builder()
    ///     .tag("landscape")?
    ///     .limit(100)
    ///     .into_post_stream()
    ///     .max_posts(250);
    ///
    /// while let Some(post_result) = stream.next().await {
    ///     let post = post_result?;
    ///     println!("Post #{}", post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn into_post_stream(self) -> PostStream<T> {
        PostStream::new(self)
    }
}
