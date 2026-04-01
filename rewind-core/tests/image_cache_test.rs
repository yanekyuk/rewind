use std::path::PathBuf;
use rewind_core::image_cache;

#[test]
fn cache_path_uses_appid() {
    let dir = PathBuf::from("/tmp/rewind-test-images");
    let path = image_cache::hero_cache_path(&dir, 12345);
    assert_eq!(path, dir.join("12345_hero.jpg"));
}

#[tokio::test]
async fn load_cached_returns_none_when_missing() {
    let dir = PathBuf::from("/tmp/rewind-test-nonexistent-dir");
    let result = image_cache::load_cached_hero(&dir, 99999);
    assert!(result.is_none());
}
