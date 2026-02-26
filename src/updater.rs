const GITHUB_REPO: &str = "dennismysh/gitmap";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
}

/// Check GitHub Releases API for a newer version.
/// Returns None if up to date or on any error.
pub fn check_for_update() -> Option<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .header("User-Agent", "gitmap-updater")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .ok()?;

    let body = response.into_body().read_to_string().ok()?;

    let json: serde_json::Value = serde_json::from_str(&body).ok()?;

    let tag = json["tag_name"].as_str()?;
    let remote_version = tag.strip_prefix('v').unwrap_or(tag);

    if remote_version <= CURRENT_VERSION {
        return None;
    }

    // Find the .zip asset
    let assets = json["assets"].as_array()?;
    let asset = assets.iter().find(|a| {
        a["name"]
            .as_str()
            .map(|n| n.ends_with("-macos-universal.zip"))
            .unwrap_or(false)
    })?;

    let download_url = asset["browser_download_url"].as_str()?;

    Some(UpdateInfo {
        version: remote_version.to_string(),
        download_url: download_url.to_string(),
    })
}
