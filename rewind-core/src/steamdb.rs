/// Returns the SteamDB depot manifests page URL for a given depot ID.
/// User opens this in their browser to find target manifest IDs.
pub fn depot_manifests_url(depot_id: u32) -> String {
    format!("https://www.steamdb.info/depot/{}/manifests/", depot_id)
}

/// Returns the SteamDB app page URL for a given app ID.
pub fn app_url(app_id: u32) -> String {
    format!("https://www.steamdb.info/app/{}/", app_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depot_manifests_url_test() {
        let url = depot_manifests_url(3321461);
        assert_eq!(url, "https://www.steamdb.info/depot/3321461/manifests/");
    }

    #[test]
    fn app_page_url_test() {
        let url = app_url(3321460);
        assert_eq!(url, "https://www.steamdb.info/app/3321460/");
    }
}