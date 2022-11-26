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
let posts = GelbooruClient::builder()
    .tag("kafuu_chino".to_string())
    .tag("2girls".to_string())
    .rating(GelbooruRating::General)
    .sort(GelbooruSort::Score)
    .limit(5)
    .random(true)
    .blacklist_tag(GelbooruRating::Explicit.to_string())
    .get()
    .await
    .expect("There was an error retrieving posts from the API");
```
