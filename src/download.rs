//! Image download utilities.
//!
//! This module provides helpers for downloading images from booru posts,
//! with support for progress tracking and concurrent downloads.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::download::{Downloader, DownloadOptions};
//! use booru_rs::prelude::*;
//! use std::path::Path;
//!
//! # async fn example() -> booru_rs::error::Result<()> {
//! let posts = SafebooruClient::builder()
//!     .tag("landscape")?
//!     .limit(5)
//!     .build()
//!     .get()
//!     .await?;
//!
//! let downloader = Downloader::new();
//!
//! for post in &posts {
//!     let path = downloader
//!         .download_post(post, Path::new("./downloads"))
//!         .await?;
//!     println!("Downloaded: {}", path.path.display());
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{BooruError, Result};
use crate::model::Post;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// Options for configuring downloads.
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// Whether to overwrite existing files.
    pub overwrite: bool,
    /// Custom filename template. Use `{id}`, `{md5}`, `{ext}` as placeholders.
    pub filename_template: Option<String>,
    /// Create subdirectories based on rating.
    pub organize_by_rating: bool,
}

impl DownloadOptions {
    /// Create options that overwrite existing files.
    #[must_use]
    pub fn overwrite(mut self) -> Self {
        self.overwrite = true;
        self
    }

    /// Set a custom filename template.
    ///
    /// Available placeholders:
    /// - `{id}` - Post ID
    /// - `{md5}` - MD5 hash (if available)
    /// - `{ext}` - File extension
    #[must_use]
    pub fn filename(mut self, template: impl Into<String>) -> Self {
        self.filename_template = Some(template.into());
        self
    }

    /// Organize downloads into subdirectories by rating.
    #[must_use]
    pub fn organize_by_rating(mut self) -> Self {
        self.organize_by_rating = true;
        self
    }
}

/// Result of a download operation.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Path where the file was saved.
    pub path: PathBuf,
    /// Size of the downloaded file in bytes.
    pub size: u64,
    /// Whether the file already existed and was skipped.
    pub skipped: bool,
}

/// Progress information for a download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Total bytes to download (if known).
    pub total: Option<u64>,
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Post ID being downloaded.
    pub post_id: u32,
}

/// A callback type for progress updates.
pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Image downloader with configurable options.
///
/// # Example
///
/// ```no_run
/// use booru_rs::download::Downloader;
/// use std::path::Path;
///
/// let downloader = Downloader::new()
///     .with_timeout(std::time::Duration::from_secs(120));
/// ```
#[derive(Clone)]
pub struct Downloader {
    client: reqwest::Client,
    options: DownloadOptions,
}

impl std::fmt::Debug for Downloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Downloader")
            .field("options", &self.options)
            .finish()
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader {
    /// Creates a new downloader with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to create HTTP client"),
            options: DownloadOptions::default(),
        }
    }

    /// Creates a new downloader with a custom HTTP client.
    #[must_use]
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            options: DownloadOptions::default(),
        }
    }

    /// Sets the download options.
    #[must_use]
    pub fn options(mut self, options: DownloadOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets a custom timeout for downloads.
    #[must_use]
    pub fn with_timeout(self, timeout: std::time::Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("Failed to create HTTP client"),
            options: self.options,
        }
    }

    /// Downloads an image from a URL to a directory.
    ///
    /// Returns the path where the file was saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails or the file cannot be written.
    pub async fn download_url(
        &self,
        url: &str,
        dest_dir: &Path,
        filename: Option<&str>,
    ) -> Result<DownloadResult> {
        // Extract filename from URL if not provided
        let filename = match filename {
            Some(f) => f.to_string(),
            None => url
                .rsplit('/')
                .next()
                .ok_or_else(|| BooruError::InvalidUrl(url.to_string()))?
                .to_string(),
        };

        let dest_path = dest_dir.join(&filename);

        // Check if file exists
        if dest_path.exists() && !self.options.overwrite {
            let metadata = tokio::fs::metadata(&dest_path).await?;
            return Ok(DownloadResult {
                path: dest_path,
                size: metadata.len(),
                skipped: true,
            });
        }

        // Create destination directory
        tokio::fs::create_dir_all(dest_dir).await?;

        // Download the file
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .map_err(BooruError::Request)?;

        let bytes = response.bytes().await?;
        let size = bytes.len() as u64;

        // Write to file
        let mut file = tokio::fs::File::create(&dest_path).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;

        Ok(DownloadResult {
            path: dest_path,
            size,
            skipped: false,
        })
    }

    /// Downloads an image from a URL with progress updates.
    ///
    /// The callback is called periodically with progress information.
    pub async fn download_url_with_progress<F>(
        &self,
        url: &str,
        dest_dir: &Path,
        filename: Option<&str>,
        post_id: u32,
        on_progress: F,
    ) -> Result<DownloadResult>
    where
        F: Fn(DownloadProgress) + Send,
    {
        let filename = match filename {
            Some(f) => f.to_string(),
            None => url
                .rsplit('/')
                .next()
                .ok_or_else(|| BooruError::InvalidUrl(url.to_string()))?
                .to_string(),
        };

        let dest_path = dest_dir.join(&filename);

        if dest_path.exists() && !self.options.overwrite {
            let metadata = tokio::fs::metadata(&dest_path).await?;
            return Ok(DownloadResult {
                path: dest_path,
                size: metadata.len(),
                skipped: true,
            });
        }

        tokio::fs::create_dir_all(dest_dir).await?;

        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .map_err(BooruError::Request)?;

        let total = response.content_length();
        let mut downloaded: u64 = 0;

        let mut file = tokio::fs::File::create(&dest_path).await?;
        let mut stream = response.bytes_stream();

        use futures_core::Stream;
        use std::pin::Pin;
        use std::task::Context;

        // Consume stream manually to track progress
        let mut stream = Pin::new(&mut stream);
        loop {
            let chunk =
                std::future::poll_fn(|cx: &mut Context<'_>| stream.as_mut().poll_next(cx)).await;

            match chunk {
                Some(Ok(bytes)) => {
                    file.write_all(&bytes).await?;
                    downloaded += bytes.len() as u64;

                    on_progress(DownloadProgress {
                        total,
                        downloaded,
                        post_id,
                    });
                }
                Some(Err(e)) => return Err(BooruError::Request(e)),
                None => break,
            }
        }

        file.flush().await?;

        Ok(DownloadResult {
            path: dest_path,
            size: downloaded,
            skipped: false,
        })
    }

    /// Downloads an image from a post.
    ///
    /// Uses the post's file URL and generates a filename based on the post ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the post has no file URL or the download fails.
    pub async fn download_post(&self, post: &impl Post, dest_dir: &Path) -> Result<DownloadResult> {
        let url = post
            .file_url()
            .ok_or_else(|| BooruError::InvalidUrl("Post has no file URL".to_string()))?;

        let filename = self.generate_filename(post, url);
        self.download_url(url, dest_dir, Some(&filename)).await
    }

    /// Downloads an image from a post with progress updates.
    pub async fn download_post_with_progress<F>(
        &self,
        post: &impl Post,
        dest_dir: &Path,
        on_progress: F,
    ) -> Result<DownloadResult>
    where
        F: Fn(DownloadProgress) + Send,
    {
        let url = post
            .file_url()
            .ok_or_else(|| BooruError::InvalidUrl("Post has no file URL".to_string()))?;

        let filename = self.generate_filename(post, url);
        self.download_url_with_progress(url, dest_dir, Some(&filename), post.id(), on_progress)
            .await
    }

    /// Downloads multiple posts concurrently.
    ///
    /// Returns results in the same order as the input posts.
    pub async fn download_posts(
        &self,
        posts: &[impl Post + Sync],
        dest_dir: &Path,
        concurrency: usize,
    ) -> Vec<Result<DownloadResult>> {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(posts.len());

        for post in posts {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let url = post.file_url().map(|s| s.to_string());
            let id = post.id();
            let filename = url.as_ref().map(|u| self.generate_filename(post, u));
            let dest = dest_dir.to_path_buf();
            let client = self.client.clone();
            let options = self.options.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;

                let url = url.ok_or_else(|| {
                    BooruError::InvalidUrl(format!("Post {} has no file URL", id))
                })?;

                let filename = filename.unwrap();
                let dest_path = dest.join(&filename);

                if dest_path.exists() && !options.overwrite {
                    let metadata = tokio::fs::metadata(&dest_path).await?;
                    return Ok(DownloadResult {
                        path: dest_path,
                        size: metadata.len(),
                        skipped: true,
                    });
                }

                tokio::fs::create_dir_all(&dest).await?;

                let response = client
                    .get(&url)
                    .send()
                    .await?
                    .error_for_status()
                    .map_err(BooruError::Request)?;

                let bytes = response.bytes().await?;
                let size = bytes.len() as u64;

                let mut file = tokio::fs::File::create(&dest_path).await?;
                file.write_all(&bytes).await?;
                file.flush().await?;

                Ok(DownloadResult {
                    path: dest_path,
                    size,
                    skipped: false,
                })
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(
                handle.await.unwrap_or_else(|e| {
                    Err(BooruError::InvalidUrl(format!("Task panicked: {}", e)))
                }),
            );
        }
        results
    }

    fn generate_filename(&self, post: &impl Post, url: &str) -> String {
        let ext = url
            .rsplit('.')
            .next()
            .unwrap_or("jpg")
            .split('?')
            .next()
            .unwrap_or("jpg");

        if let Some(template) = &self.options.filename_template {
            let mut filename = template.clone();
            filename = filename.replace("{id}", &post.id().to_string());
            filename = filename.replace("{md5}", post.md5().unwrap_or("unknown"));
            filename = filename.replace("{ext}", ext);
            filename
        } else {
            format!("{}.{}", post.id(), ext)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_options_default() {
        let opts = DownloadOptions::default();
        assert!(!opts.overwrite);
        assert!(opts.filename_template.is_none());
    }

    #[test]
    fn test_download_options_builder() {
        let opts = DownloadOptions::default()
            .overwrite()
            .filename("{id}_{md5}.{ext}".to_string());

        assert!(opts.overwrite);
        assert!(opts.filename_template.is_some());
    }
}
