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

/// Download the update zip and replace /Applications/GitMap.app.
pub fn download_and_install(download_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = std::path::Path::new("/tmp/gitmap-update");

    // Clean up any previous update attempt
    if tmp_dir.exists() {
        std::fs::remove_dir_all(tmp_dir)?;
    }
    std::fs::create_dir_all(tmp_dir)?;

    let zip_path = tmp_dir.join("GitMap.zip");

    // Download the zip
    let response = ureq::get(download_url)
        .header("User-Agent", "gitmap-updater")
        .call()?;

    let mut bytes = Vec::new();
    use std::io::Read;
    response.into_body().into_reader().read_to_end(&mut bytes)?;
    std::fs::write(&zip_path, &bytes)?;

    // Extract using ditto (macOS built-in, preserves attributes)
    let status = std::process::Command::new("ditto")
        .args(["-xk", &zip_path.to_string_lossy(), &tmp_dir.to_string_lossy()])
        .status()?;

    if !status.success() {
        return Err("ditto extraction failed".into());
    }

    let extracted_app = tmp_dir.join("GitMap.app");
    if !extracted_app.exists() {
        return Err("GitMap.app not found in zip".into());
    }

    // Replace the installed app
    let installed_app = std::path::Path::new("/Applications/GitMap.app");
    if installed_app.exists() {
        std::fs::remove_dir_all(installed_app)?;
    }

    let status = std::process::Command::new("mv")
        .args([
            extracted_app.to_string_lossy().to_string(),
            "/Applications/GitMap.app".to_string(),
        ])
        .status()?;

    if !status.success() {
        return Err("failed to move GitMap.app to /Applications".into());
    }

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(tmp_dir);

    Ok(())
}
