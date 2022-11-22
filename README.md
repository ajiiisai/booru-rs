# booru-rs
An async Booru client for Rust

##  Overview
The client currently supports:
- [x] Gelbooru
- [ ] Safebooru
- [ ] Danbooru
- [ ] Konachan
- [ ] R34
- [ ] 3DBooru
- [ ] More... ?

## Example
```rust
let client = GelbooruClient::new(None);
let posts = client.get_posts_by_tag("kafuu_chino rating:general").await.unwrap();
```
Another way of  inputing the rating is using the `GelbooruRating` enum
```rust
let client = GelbooruClient::new(None);
let posts = client.get_posts_by_tag_and_rating("kafuu_chino", GelbooruRating::General).await.unwrap() ;
```
