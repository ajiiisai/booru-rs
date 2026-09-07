//! Image download utilities.
//!
//! This module provides helpers for downloading images from booru posts,
//! with support for progress tracking and concurrent downloads.
//! Responses labeled `text/html` or `application/xhtml+xml` are rejected before
//! writing a file. Other content types, including missing headers, are accepted.
//!
//! # Example
//!
//! ```no_run
//! use booru_rs::download::Downloader;
//! use std::path::Path;
//!
//! # async fn example() -> booru_rs::error::Result<()> {
//! let downloader = Downloader::new();
//! let result = downloader
//!     .download_url(
//!         "https://example.com/image.jpg",
//!         Path::new("./downloads"),
//!         None,
//!     )
//!     .await?;
//! println!("Downloaded: {}", result.path.display());
//! # Ok(())
//! # }
//! ```

use crate::error::{BooruError, Result};
use crate::model::Post;
use reqwest::header::{CONTENT_TYPE, HeaderMap};
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

fn is_rejected_download_content_type(content_type: &str) -> bool {
    let media_type = content_type.split(';').next().unwrap_or_default().trim();
    if media_type.is_empty() || media_type.eq_ignore_ascii_case("image/svg+xml") {
        return false;
    }
    let lower = media_type.to_ascii_lowercase();
    lower.starts_with("text/")
        || lower == "application/json"
        || lower == "application/xml"
        || lower == "application/xhtml+xml"
        || lower.ends_with("+xml")
        || lower.contains("html")
}

fn extension_from_url(url: &str) -> String {
    if let Ok(parsed) = reqwest::Url::parse(url)
        && let Some(mut segments) = parsed.path_segments()
        && let Some(last) = segments.rfind(|segment| !segment.is_empty())
        && let Some(dot) = last.rfind('.')
    {
        let ext = &last[dot + 1..];
        if !ext.is_empty() && ext.len() <= 10 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            return ext.to_string();
        }
        return "jpg".to_string();
    }
    "jpg".to_string()
}

async fn stream_response_to_file(
    response: reqwest::Response,
    dest_path: &Path,
    overwrite: bool,
    post_id: Option<u32>,
    on_progress: Option<&(dyn Fn(DownloadProgress) + Send + Sync)>,
) -> Result<DownloadResult> {
    use futures_core::Stream;

    if let Some(content_type) = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        && is_rejected_download_content_type(content_type)
    {
        return Err(BooruError::UnexpectedDownloadContentType(
            content_type.to_string(),
        ));
    }

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
            Some(Err(error)) => return Err(error.into()),
            None => break,
        }
    }

    file.flush().await?;
    drop(file);
    if overwrite {
        temp.persist(dest_path)
            .map_err(|error| BooruError::Io(error.error))?;
    } else if let Err(error) = temp.persist_noclobber(dest_path) {
        if error.error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(BooruError::Io(error.error));
        }
        let metadata = tokio::fs::metadata(dest_path).await?;
        return Ok(DownloadResult {
            path: dest_path.to_path_buf(),
            size: metadata.len(),
            skipped: true,
        });
    }
    Ok(DownloadResult {
        path: dest_path.to_path_buf(),
        size: downloaded,
        skipped: false,
    })
}

/// Options for configuring downloads.
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// Whether to overwrite existing files.
    pub overwrite: bool,
    /// Custom filename template. Use `{id}`, `{md5}`, `{ext}` as placeholders.
    pub filename_template: Option<String>,
    /// Whether to verify downloaded bytes against the post MD5.
    ///
    /// Disabled by default. When enabled, posts without MD5 metadata still
    /// download without verification, and already skipped files are not read.
    pub verify_md5: bool,
}

async fn verify_download_md5(path: &Path, id: u32, expected: &str) -> Result<()> {
    let bytes = tokio::fs::read(path).await?;
    let actual = format!("{:x}", md5::compute(&bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        let _ = tokio::fs::remove_file(path).await;
        Err(BooruError::Md5Mismatch {
            id,
            expected: expected.to_string(),
            actual,
        })
    }
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

    /// Verify downloaded bytes against the post MD5 hash when available.
    ///
    /// Mismatched files are removed and reported as
    /// `BooruError::Md5Mismatch`. Files that already existed and were
    /// skipped are not read.
    #[must_use]
    pub fn verify_md5(mut self) -> Self {
        self.verify_md5 = true;
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
    headers: HeaderMap,
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
    const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

    /// Creates a new downloader with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            headers: HeaderMap::new(),
            options: DownloadOptions::default(),
            timeout: Some(Self::DEFAULT_TIMEOUT),
        }
    }

    /// Creates a new downloader with a custom HTTP client.
    #[must_use]
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            headers: HeaderMap::new(),
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

    /// Sets headers for every download request, including batch downloads.
    ///
    /// Replaces headers set by a previous call to this method. These headers
    /// override matching default headers on a custom HTTP client.
    ///
    /// # Example
    ///
    /// Gelbooru's image servers can require a Referer header:
    ///
    /// ```
    /// use booru_rs::download::Downloader;
    /// use reqwest::header::{HeaderMap, HeaderValue, REFERER};
    ///
    /// let mut headers = HeaderMap::new();
    /// headers.insert(REFERER, HeaderValue::from_static("https://gelbooru.com"));
    /// let downloader = Downloader::new().with_headers(headers);
    /// ```
    #[must_use]
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Sets a custom timeout for downloads.
    #[must_use]
    pub fn with_timeout(self, timeout: std::time::Duration) -> Self {
        Self {
            client: self.client,
            headers: self.headers,
            options: self.options,
            timeout: Some(timeout),
        }
    }

    fn configure_request(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let request = request.headers(self.headers.clone());
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
        if tokio::fs::try_exists(&dest_path).await? && !self.options.overwrite {
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
            .configure_request(self.client.get(url))
            .send()
            .await?
            .error_for_status()
            .map_err(BooruError::from)?;

        stream_response_to_file(response, &dest_path, self.options.overwrite, None, None).await
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

        if tokio::fs::try_exists(&dest_path).await? && !self.options.overwrite {
            let metadata = tokio::fs::metadata(&dest_path).await?;
            return Ok(DownloadResult {
                path: dest_path,
                size: metadata.len(),
                skipped: true,
            });
        }

        tokio::fs::create_dir_all(dest_dir).await?;

        let response = self
            .configure_request(self.client.get(url))
            .send()
            .await?
            .error_for_status()
            .map_err(BooruError::from)?;

        stream_response_to_file(
            response,
            &dest_path,
            self.options.overwrite,
            Some(post_id),
            Some(&on_progress),
        )
        .await
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
        let result = self.download_url(url, dest_dir, Some(&filename)).await?;
        if self.options.verify_md5
            && !result.skipped
            && let Some(expected) = post.md5()
        {
            verify_download_md5(&result.path, post.id(), expected).await?;
        }
        Ok(result)
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
        let result = self
            .download_url_with_progress(url, dest_dir, Some(&filename), post.id(), on_progress)
            .await?;
        if self.options.verify_md5
            && !result.skipped
            && let Some(expected) = post.md5()
        {
            verify_download_md5(&result.path, post.id(), expected).await?;
        }
        Ok(result)
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
        self.download_posts_inner(posts, dest_dir, concurrency, None)
            .await
    }

    /// Downloads multiple posts concurrently with progress updates.
    ///
    /// Returns results in the same order as the input posts.
    pub async fn download_posts_with_progress<F>(
        &self,
        posts: &[impl Post + Sync],
        dest_dir: &Path,
        concurrency: usize,
        on_progress: F,
    ) -> Vec<Result<DownloadResult>>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let progress: std::sync::Arc<dyn Fn(DownloadProgress) + Send + Sync> =
            std::sync::Arc::new(on_progress);
        self.download_posts_inner(posts, dest_dir, concurrency, Some(progress))
            .await
    }

    async fn download_posts_inner(
        &self,
        posts: &[impl Post + Sync],
        dest_dir: &Path,
        concurrency: usize,
        progress: Option<std::sync::Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> Vec<Result<DownloadResult>> {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        if concurrency == 0 {
            return (0..posts.len())
                .map(|_| Err(BooruError::InvalidConcurrency))
                .collect();
        }

        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut tasks: tokio::task::JoinSet<(usize, Result<DownloadResult>)> =
            tokio::task::JoinSet::new();
        let mut task_indexes = HashMap::new();
        let mut destinations: HashMap<std::path::PathBuf, Vec<usize>> = HashMap::new();
        let mut filenames = vec![None; posts.len()];
        let mut results: Vec<Option<Result<DownloadResult>>> =
            (0..posts.len()).map(|_| None).collect();

        for (index, post) in posts.iter().enumerate() {
            let Some(url) = post.file_url() else {
                results[index] = Some(Err(BooruError::MissingMediaUrl(post.id())));
                continue;
            };
            let filename = self.generate_filename(post, url);
            if let Err(error) = validate_filename(&filename) {
                results[index] = Some(Err(error));
                continue;
            }
            destinations
                .entry(dest_dir.join(&filename))
                .or_default()
                .push(index);
            filenames[index] = Some(filename);
        }
        let conflicts: HashMap<usize, std::path::PathBuf> = destinations
            .into_iter()
            .filter(|(_, indexes)| indexes.len() > 1)
            .flat_map(|(destination, indexes)| {
                indexes
                    .into_iter()
                    .map(move |index| (index, destination.clone()))
            })
            .collect();

        for (index, post) in posts.iter().enumerate() {
            if results[index].is_some() || conflicts.contains_key(&index) {
                continue;
            }
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let url = post.file_url().unwrap().to_string();
            let post_id = post.id();
            let expected_md5 = post.md5().map(str::to_string);
            let filename = filenames[index].take().unwrap();
            let dest = dest_dir.to_path_buf();
            let client = self.client.clone();
            let options = self.options.clone();
            let timeout = self.timeout;
            let headers = self.headers.clone();
            let progress = progress.clone();

            let task = tasks.spawn(async move {
                let _permit = permit;
                let result = async {
                    let dest_path = dest.join(&filename);

                    if tokio::fs::try_exists(&dest_path).await? && !options.overwrite {
                        let metadata = tokio::fs::metadata(&dest_path).await?;
                        return Ok(DownloadResult {
                            path: dest_path,
                            size: metadata.len(),
                            skipped: true,
                        });
                    }

                    tokio::fs::create_dir_all(&dest).await?;

                    let request = client.get(&url).headers(headers);
                    let request = match timeout {
                        Some(timeout) => request.timeout(timeout),
                        None => request,
                    };
                    let response = request
                        .send()
                        .await?
                        .error_for_status()
                        .map_err(BooruError::from)?;

                    let result = stream_response_to_file(
                        response,
                        &dest_path,
                        options.overwrite,
                        Some(post_id),
                        progress.as_deref(),
                    )
                    .await?;
                    if options.verify_md5
                        && !result.skipped
                        && let Some(expected) = &expected_md5
                    {
                        verify_download_md5(&result.path, post_id, expected).await?;
                    }
                    Ok(result)
                }
                .await;
                (index, result)
            });
            task_indexes.insert(task.id(), index);
        }

        for (index, destination) in conflicts {
            results[index] = Some(Err(BooruError::DestinationConflict(destination)));
        }
        while let Some(result) = tasks.join_next_with_id().await {
            match result {
                Ok((task_id, (index, result))) => {
                    task_indexes.remove(&task_id);
                    results[index] = Some(result);
                }
                Err(error) => {
                    if let Some(index) = task_indexes.remove(&error.id()) {
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
        let ext = extension_from_url(url);

        if let Some(template) = &self.options.filename_template {
            let mut filename = template.clone();
            filename = filename.replace("{id}", &post.id().to_string());
            filename = filename.replace("{md5}", post.md5().unwrap_or("unknown"));
            filename = filename.replace("{ext}", &ext);
            filename
        } else {
            format!("{}.{}", post.id(), ext)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DownloadPost(String);

    impl Post for DownloadPost {
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

    async fn download_using(
        downloader: &Downloader,
        url: &str,
        dest: &Path,
        mode: u8,
    ) -> Result<DownloadResult> {
        match mode {
            0 => downloader.download_url(url, dest, None).await,
            1 => {
                downloader
                    .download_url_with_progress(url, dest, None, 1, |_| {})
                    .await
            }
            2 => downloader
                .download_posts(&[DownloadPost(url.to_string())], dest, 1)
                .await
                .remove(0),
            3 => downloader
                .download_posts_with_progress(&[DownloadPost(url.to_string())], dest, 1, |_| {})
                .await
                .remove(0),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn all_download_paths_reject_html_without_creating_files() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        for content_type in [
            "text/html",
            "Text/HTML; charset=UTF-8",
            "application/xhtml+xml",
            "text/plain",
            "application/json",
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_raw("<html>post page</html>", content_type),
                )
                .mount(&server)
                .await;
            for mode in 0..4 {
                let dest = tempfile::tempdir().unwrap();
                let error = download_using(
                    &Downloader::new(),
                    &format!("{}/image.jpg", server.uri()),
                    dest.path(),
                    mode,
                )
                .await
                .unwrap_err();
                assert!(
                    matches!(error, BooruError::UnexpectedDownloadContentType(value) if value == content_type)
                );
                assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
            }
        }
    }

    #[tokio::test]
    async fn all_download_paths_send_headers_and_preserve_client_defaults() {
        use reqwest::header::{HeaderValue, REFERER};
        use wiremock::matchers::{header, method};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(header("referer", "https://gelbooru.com"))
            .and(header("x-client-default", "kept"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(b"image bytes".to_vec(), "image/jpeg"),
            )
            .expect(4)
            .mount(&server)
            .await;
        let mut defaults = HeaderMap::new();
        defaults.insert("x-client-default", HeaderValue::from_static("kept"));
        defaults.insert(REFERER, HeaderValue::from_static("https://example.com"));
        let client = reqwest::Client::builder()
            .default_headers(defaults)
            .build()
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(REFERER, HeaderValue::from_static("https://gelbooru.com"));
        let downloader = Downloader::with_client(client)
            .with_headers(headers)
            .with_timeout(std::time::Duration::from_secs(7));
        for mode in 0..4 {
            let dest = tempfile::tempdir().unwrap();
            let result = download_using(
                &downloader,
                &format!("{}/image.jpg", server.uri()),
                dest.path(),
                mode,
            )
            .await
            .unwrap();
            assert_eq!(std::fs::read(result.path).unwrap(), b"image bytes");
        }
    }

    #[tokio::test]
    async fn redirected_html_is_rejected_without_overwriting_file() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(path("/image.jpg"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/post"))
            .mount(&server)
            .await;
        Mock::given(path("/post"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw("<html>post page</html>", "text/html; charset=utf-8"),
            )
            .mount(&server)
            .await;
        let dest = tempfile::tempdir().unwrap();
        let target = dest.path().join("image.jpg");
        std::fs::write(&target, b"existing image").unwrap();
        let result = Downloader::new()
            .options(DownloadOptions::default().overwrite())
            .download_url(&format!("{}/image.jpg", server.uri()), dest.path(), None)
            .await;
        assert!(matches!(
            result,
            Err(BooruError::UnexpectedDownloadContentType(_))
        ));
        assert_eq!(std::fs::read(target).unwrap(), b"existing image");
        assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejected_content_types_cover_text_and_data() {
        for content_type in [
            "text/html",
            "text/plain; charset=utf-8",
            "application/json",
            "application/xml",
            "application/xhtml+xml",
        ] {
            assert!(
                is_rejected_download_content_type(content_type),
                "{content_type}"
            );
        }
        for content_type in [
            "image/jpeg",
            "video/mp4",
            "application/octet-stream",
            "image/svg+xml; charset=utf-8",
        ] {
            assert!(
                !is_rejected_download_content_type(content_type),
                "{content_type}"
            );
        }
    }

    #[test]
    fn extension_falls_back_to_jpg_without_path_extension() {
        assert_eq!(
            extension_from_url("https://example.com/media/image.jpg"),
            "jpg"
        );
        assert_eq!(
            extension_from_url("https://example.com/media/image.png?token=secret"),
            "png"
        );
        assert_eq!(extension_from_url("https://example.com/image"), "jpg");
        assert_eq!(
            extension_from_url("https://example.com/file?md5=abc"),
            "jpg"
        );
    }

    #[test]
    fn test_download_options_default() {
        let opts = DownloadOptions::default();
        assert!(!opts.overwrite);
        assert!(opts.filename_template.is_none());
        assert!(!opts.verify_md5);
    }

    #[test]
    fn test_download_options_builder() {
        let opts = DownloadOptions::default()
            .overwrite()
            .filename("{id}_{md5}.{ext}".to_string())
            .verify_md5();

        assert!(opts.overwrite);
        assert!(opts.filename_template.is_some());
        assert!(opts.verify_md5);
    }

    #[test]
    fn default_downloader_applies_default_timeout() {
        assert_eq!(
            Downloader::new().timeout,
            Some(std::time::Duration::from_secs(300))
        );
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
    async fn no_overwrite_preserves_file_created_during_download() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/image.jpg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]))
            .mount(&server)
            .await;
        let destination = tempfile::tempdir().unwrap();
        let target = destination.path().join("image.jpg");
        let callback_target = target.clone();

        let result = Downloader::new()
            .download_url_with_progress(
                &format!("{}/image.jpg", server.uri()),
                destination.path(),
                None,
                1,
                move |_| std::fs::write(&callback_target, b"existing").unwrap(),
            )
            .await
            .unwrap();

        assert!(result.skipped);
        assert_eq!(result.size, 8);
        assert_eq!(std::fs::read(target).unwrap(), b"existing");
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
    async fn batch_rejects_destination_collisions() {
        struct SameDestinationPost;

        impl Post for SameDestinationPost {
            fn id(&self) -> u32 {
                7
            }
            fn width(&self) -> u32 {
                1
            }
            fn height(&self) -> Option<u32> {
                Some(1)
            }
            fn file_url(&self) -> Option<&str> {
                Some("https://example.com/image.jpg")
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

        let posts = [SameDestinationPost, SameDestinationPost];
        let results = Downloader::new()
            .download_posts(&posts, Path::new("downloads"), 2)
            .await;
        assert!(matches!(
            results.as_slice(),
            [
                Err(BooruError::DestinationConflict(_)),
                Err(BooruError::DestinationConflict(_))
            ]
        ));
    }

    #[tokio::test]
    async fn batch_rejects_generated_path_components() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        struct TestPost(String);

        impl Post for TestPost {
            fn id(&self) -> u32 {
                7
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
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]))
            .mount(&server)
            .await;
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("downloads");
        let posts = [TestPost(format!("{}/image.jpg", server.uri()))];
        let results = Downloader::new()
            .options(DownloadOptions::default().filename("../escaped.jpg"))
            .download_posts(&posts, &destination, 1)
            .await;

        assert!(matches!(
            results.as_slice(),
            [Err(BooruError::InvalidFilename(filename))] if filename == "../escaped.jpg"
        ));
        assert!(!root.path().join("escaped.jpg").exists());
    }

    #[tokio::test]
    async fn batch_keeps_failures_in_input_order() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        struct TestPost {
            id: u32,
            url: Option<String>,
        }

        impl Post for TestPost {
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
                self.url.as_deref()
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
            .and(path("/slow.jpg"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(vec![1, 2, 3])
                    .set_delay(std::time::Duration::from_millis(50)),
            )
            .mount(&server)
            .await;
        let destination = tempfile::tempdir().unwrap();
        let posts = [
            TestPost {
                id: 1,
                url: Some(format!("{}/slow.jpg", server.uri())),
            },
            TestPost { id: 2, url: None },
        ];
        let results = Downloader::new()
            .download_posts(&posts, destination.path(), 2)
            .await;

        assert!(results[0].is_ok());
        assert!(matches!(results[1], Err(BooruError::MissingMediaUrl(2))));
    }

    #[tokio::test]
    async fn batch_progress_reports_post_ids_in_input_order() {
        use std::sync::{Arc, Mutex};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        struct TestPost {
            id: u32,
            url: String,
        }

        impl Post for TestPost {
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
                Some(&self.url)
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
            .and(path("/a.jpg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/b.jpg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![4, 5]))
            .mount(&server)
            .await;
        let destination = tempfile::tempdir().unwrap();
        let posts = [
            TestPost {
                id: 7,
                url: format!("{}/a.jpg", server.uri()),
            },
            TestPost {
                id: 9,
                url: format!("{}/b.jpg", server.uri()),
            },
        ];
        let seen: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
        let seen_task = seen.clone();
        let results = Downloader::new()
            .download_posts_with_progress(&posts, destination.path(), 2, move |progress| {
                seen_task.lock().unwrap().push(progress.post_id);
            })
            .await;

        assert!(results.iter().all(|result| result.is_ok()));
        let mut seen = seen.lock().unwrap().clone();
        seen.sort_unstable();
        assert_eq!(seen, vec![7, 9]);
    }

    #[tokio::test]
    async fn md5_verification_accepts_match_and_rejects_mismatch() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        struct Md5Post {
            id: u32,
            url: String,
            md5: Option<String>,
        }

        impl Post for Md5Post {
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
                Some(&self.url)
            }
            fn tags(&self) -> &str {
                ""
            }
            fn score(&self) -> Option<i64> {
                None
            }
            fn md5(&self) -> Option<&str> {
                self.md5.as_deref()
            }
            fn source(&self) -> Option<&str> {
                None
            }
        }

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(b"image bytes".to_vec(), "image/jpeg"),
            )
            .mount(&server)
            .await;
        let matching = "bebb32c1d5592c44df47d1826cacc09b".to_string();

        let dest = tempfile::tempdir().unwrap();
        let post = Md5Post {
            id: 1,
            url: format!("{}/a.jpg", server.uri()),
            md5: Some(matching.clone()),
        };
        let result = Downloader::new()
            .options(DownloadOptions::default().verify_md5())
            .download_post(&post, dest.path())
            .await
            .unwrap();
        assert!(!result.skipped);
        assert_eq!(std::fs::read(&result.path).unwrap(), b"image bytes");

        let dest = tempfile::tempdir().unwrap();
        let post = Md5Post {
            id: 2,
            url: format!("{}/b.jpg", server.uri()),
            md5: Some("00000000000000000000000000000000".to_string()),
        };
        let error = Downloader::new()
            .options(DownloadOptions::default().verify_md5())
            .download_post(&post, dest.path())
            .await
            .unwrap_err();
        assert!(matches!(error, BooruError::Md5Mismatch { id: 2, .. }));
        assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);

        let dest = tempfile::tempdir().unwrap();
        let post = Md5Post {
            id: 3,
            url: format!("{}/c.jpg", server.uri()),
            md5: Some("00000000000000000000000000000000".to_string()),
        };
        let result = Downloader::new()
            .download_post(&post, dest.path())
            .await
            .unwrap();
        assert!(!result.skipped);

        let dest = tempfile::tempdir().unwrap();
        let posts = [
            Md5Post {
                id: 4,
                url: format!("{}/d.jpg", server.uri()),
                md5: Some(matching),
            },
            Md5Post {
                id: 5,
                url: format!("{}/e.jpg", server.uri()),
                md5: None,
            },
        ];
        let results = Downloader::new()
            .options(DownloadOptions::default().verify_md5())
            .download_posts(&posts, dest.path(), 2)
            .await;
        assert!(results.iter().all(|result| result.is_ok()));
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
