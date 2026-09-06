//! Image download utilities.
//!
//! This module provides helpers for downloading images from booru posts,
//! with support for progress tracking and concurrent downloads.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::download::{Downloader, DownloadOptions};
//! use booru_rs::safebooru::Client;
//! use std::path::Path;
//!
//! # async fn example() -> booru_rs::error::Result<()> {
//! let client = Client::new()?;
//! let posts = client.search().tag("landscape").limit(5).send().await?;
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

fn validate_filename(filename: &str) -> Result<()> {
    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(BooruError::InvalidFilename(filename.to_string()));
    }
    Ok(())
}

fn filename_from_url(url: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| BooruError::InvalidUrl(url.to_string()))?;
    if parsed.path().is_empty() || parsed.path().ends_with('/') {
        return Err(BooruError::InvalidUrl(url.to_string()));
    }
    let filename = parsed
        .path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .ok_or_else(|| BooruError::InvalidUrl(url.to_string()))?;
    let filename = filename.to_string();
    validate_filename(&filename)?;
    Ok(filename)
}

async fn stream_response_to_file(
    response: reqwest::Response,
    dest_path: &Path,
    post_id: Option<u32>,
    on_progress: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
) -> Result<u64> {
    use futures_core::Stream;

    let parent = dest_path.parent().unwrap_or_else(|| Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    let mut file = tokio::fs::File::from_std(temp.reopen()?);
    let total = response.content_length();
    let mut downloaded = 0;
    let mut stream = std::pin::pin!(response.bytes_stream());

    loop {
        let chunk = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await;
        match chunk {
            Some(Ok(bytes)) => {
                file.write_all(&bytes).await?;
                downloaded += bytes.len() as u64;
                if let Some(on_progress) = on_progress
                    && let Some(post_id) = post_id
                {
                    on_progress(DownloadProgress {
                        total,
                        downloaded,
                        post_id,
                    });
                }
            }
            Some(Err(error)) => return Err(BooruError::Request(error)),
            None => break,
        }
    }

    file.flush().await?;
    drop(file);
    temp.persist(dest_path)
        .map_err(|error| BooruError::Io(error.error))?;
    Ok(downloaded)
}

/// Options for configuring downloads.
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// Whether to overwrite existing files.
    pub overwrite: bool,
    /// Custom filename template. Use `{id}`, `{md5}`, `{ext}` as placeholders.
    pub filename_template: Option<String>,
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
    timeout: Option<std::time::Duration>,
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
            timeout: None,
        }
    }

    /// Creates a new downloader with a custom HTTP client.
    #[must_use]
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            options: DownloadOptions::default(),
            timeout: None,
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
            client: self.client,
            options: self.options,
            timeout: Some(timeout),
        }
    }

    fn apply_timeout(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.timeout {
            Some(timeout) => request.timeout(timeout),
            None => request,
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
            Some(f) => {
                validate_filename(f)?;
                f.to_string()
            }
            None => filename_from_url(url)?,
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
            .apply_timeout(self.client.get(url))
            .send()
            .await?
            .error_for_status()
            .map_err(BooruError::Request)?;

        let size = stream_response_to_file(response, &dest_path, None, None).await?;

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
        F: Fn(DownloadProgress) + Send + Sync,
    {
        let filename = match filename {
            Some(f) => {
                validate_filename(f)?;
                f.to_string()
            }
            None => filename_from_url(url)?,
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
            .apply_timeout(self.client.get(url))
            .send()
            .await?
            .error_for_status()
            .map_err(BooruError::Request)?;

        let downloaded =
            stream_response_to_file(response, &dest_path, Some(post_id), Some(&on_progress))
                .await?;

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
            .ok_or_else(|| BooruError::MissingMediaUrl(post.id()))?;

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
        F: Fn(DownloadProgress) + Send + Sync,
    {
        let url = post
            .file_url()
            .ok_or_else(|| BooruError::MissingMediaUrl(post.id()))?;

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

        if concurrency == 0 {
            return (0..posts.len())
                .map(|_| Err(BooruError::InvalidConcurrency))
                .collect();
        }

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut tasks: tokio::task::JoinSet<Result<(usize, DownloadResult)>> =
            tokio::task::JoinSet::new();

        for (index, post) in posts.iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let url = post.file_url().map(|s| s.to_string());
            let id = post.id();
            let filename = url.as_ref().map(|u| self.generate_filename(post, u));
            let dest = dest_dir.to_path_buf();
            let client = self.client.clone();
            let options = self.options.clone();
            let timeout = self.timeout;

            tasks.spawn(async move {
                let _permit = permit;

                let url = url.ok_or_else(|| BooruError::MissingMediaUrl(id))?;

                let filename = filename.unwrap();
                let dest_path = dest.join(&filename);

                if dest_path.exists() && !options.overwrite {
                    let metadata = tokio::fs::metadata(&dest_path).await?;
                    return Ok((
                        index,
                        DownloadResult {
                            path: dest_path,
                            size: metadata.len(),
                            skipped: true,
                        },
                    ));
                }

                tokio::fs::create_dir_all(&dest).await?;

                let request = client.get(&url);
                let request = match timeout {
                    Some(timeout) => request.timeout(timeout),
                    None => request,
                };
                let response = request
                    .send()
                    .await?
                    .error_for_status()
                    .map_err(BooruError::Request)?;

                let size = stream_response_to_file(response, &dest_path, None, None).await?;

                Ok((
                    index,
                    DownloadResult {
                        path: dest_path,
                        size,
                        skipped: false,
                    },
                ))
            });
        }

        let mut results: Vec<Option<Result<DownloadResult>>> =
            (0..posts.len()).map(|_| None).collect();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok((index, result))) => results[index] = Some(Ok(result)),
                Ok(Err(error)) => {
                    if let Some(index) = results.iter().position(Option::is_none) {
                        results[index] = Some(Err(error));
                    }
                }
                Err(error) => {
                    if let Some(index) = results.iter().position(Option::is_none) {
                        results[index] =
                            Some(Err(BooruError::DownloadTaskFailed(error.to_string())));
                    }
                }
            }
        }
        results
            .into_iter()
            .map(|result| result.expect("every download task must return a result"))
            .collect()
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

    #[test]
    fn filename_from_url_uses_path_without_query() {
        assert_eq!(
            filename_from_url("https://example.com/media/image.jpg?token=secret").unwrap(),
            "image.jpg"
        );
    }

    #[test]
    fn filename_from_url_rejects_empty_paths() {
        for url in ["https://example.com", "https://example.com/dir/"] {
            assert!(matches!(
                filename_from_url(url),
                Err(BooruError::InvalidUrl(_))
            ));
        }
    }

    #[test]
    fn filename_validation_rejects_path_components() {
        for filename in [
            "",
            ".",
            "..",
            "../image.jpg",
            "nested/image.jpg",
            r"..\image.jpg",
        ] {
            assert!(matches!(
                validate_filename(filename),
                Err(BooruError::InvalidFilename(_))
            ));
        }
        assert!(validate_filename("image.jpg").is_ok());
    }

    #[tokio::test]
    async fn timeout_preserves_injected_client_configuration() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-test", reqwest::header::HeaderValue::from_static("kept"));
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        let downloader =
            Downloader::with_client(client).with_timeout(std::time::Duration::from_secs(7));
        Mock::given(method("GET"))
            .and(path("/image.jpg"))
            .and(header("x-test", "kept"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]))
            .mount(&server)
            .await;

        let dest =
            std::env::temp_dir().join(format!("booru-rs-download-test-{}", std::process::id()));
        let result = downloader
            .download_url(&format!("{}/image.jpg", server.uri()), &dest, None)
            .await
            .unwrap();
        assert_eq!(result.size, 3);
        assert_eq!(tokio::fs::read(&result.path).await.unwrap(), vec![1, 2, 3]);
        tokio::fs::remove_dir_all(dest).await.unwrap();
    }

    #[tokio::test]
    async fn zero_concurrency_returns_errors_without_waiting() {
        struct TestPost;

        impl Post for TestPost {
            fn id(&self) -> u32 {
                1
            }
            fn width(&self) -> u32 {
                1
            }
            fn height(&self) -> Option<u32> {
                Some(1)
            }
            fn file_url(&self) -> Option<&str> {
                Some("https://example.com/1.jpg")
            }
            fn tags(&self) -> &str {
                ""
            }
            fn score(&self) -> Option<i64> {
                Some(0)
            }
            fn md5(&self) -> Option<&str> {
                None
            }
            fn source(&self) -> Option<&str> {
                None
            }
        }

        let posts = [TestPost, TestPost];
        let results = Downloader::new()
            .download_posts(&posts, Path::new("unused"), 0)
            .await;

        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(BooruError::InvalidConcurrency)))
        );
    }

    #[tokio::test]
    async fn missing_media_url_has_download_error() {
        struct NoMediaPost;

        impl Post for NoMediaPost {
            fn id(&self) -> u32 {
                42
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

        let error = Downloader::new()
            .download_post(&NoMediaPost, Path::new("unused"))
            .await
            .unwrap_err();
        assert!(matches!(error, BooruError::MissingMediaUrl(42)));

        let results = Downloader::new()
            .download_posts(&[NoMediaPost], Path::new("unused"), 1)
            .await;
        assert!(matches!(
            results.as_slice(),
            [Err(BooruError::MissingMediaUrl(42))]
        ));
    }

    #[tokio::test]
    async fn cancelling_batch_aborts_in_flight_downloads() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        struct TestPost(String);

        impl Post for TestPost {
            fn id(&self) -> u32 {
                1
            }
            fn width(&self) -> u32 {
                1
            }
            fn height(&self) -> Option<u32> {
                Some(1)
            }
            fn file_url(&self) -> Option<&str> {
                Some(&self.0)
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

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/image.jpg"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(vec![1, 2, 3])
                    .set_delay(std::time::Duration::from_millis(200)),
            )
            .mount(&server)
            .await;

        let dest =
            std::env::temp_dir().join(format!("booru-rs-download-cancel-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&dest).await;
        let post = TestPost(format!("{}/image.jpg", server.uri()));
        let downloader = Downloader::new();
        let task_dest = dest.clone();
        let task =
            tokio::spawn(async move { downloader.download_posts(&[post], &task_dest, 1).await });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        task.abort();
        let _ = task.await;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        assert!(!dest.join("1.jpg").exists());
        let _ = tokio::fs::remove_dir_all(dest).await;
    }
}
