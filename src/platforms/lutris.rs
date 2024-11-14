#[cfg(feature = "http")]
use crate::{GameInfo, GamePlatform};
use std::fs::exists;
use anyhow::Result;
use home::home_dir;

use crate::DetectedGame;

const SQL: &str = "SELECT * FROM games";

pub fn detect_game(processes: &Vec<procfs::process::Process>) -> Result<Option<DetectedGame>> {
    let db_path = home_dir().unwrap().join(".local/share/lutris/pga.db");
    if !exists(&db_path)? {
        return Ok(None);
    }
    let db = sqlite::open(&db_path)?;
    let cursor = db.prepare(SQL)?;
    let rows = cursor.into_iter();
    for row in rows {
        let row = row?;

        let id: i64 = row.read("id");
        let slug: &str = row.read("slug");
        let name: &str = row.read("name");
        let directory: &str = row.read("directory");

        for process in processes {
            let cmd = process.cmdline().ok();
            match cmd {
                Some(cmd) => {
                    let cmd = cmd.join(" ").to_lowercase();
                    if cmd.contains("lutris-wrapper") && cmd.contains(&name.to_lowercase()) && cmd.contains(&directory) {
                        return Ok(Some(DetectedGame::Lutris {
                            id,
                            slug: slug.to_string(),
                            name: name.to_string(),
                            cover: format!("https://lutris.net/games/banner/{0}.jpg", slug),
                            icon: format!("https://lutris.net/games/icon/{0}.png", slug)
                        }))
                    }
                }
                None => continue,
            }
        }
    }
    Ok(None)
}

#[cfg(feature = "http")]
mod http {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(crate) struct Response {
        #[serde(rename = "steamid")]
        pub steam_id: Option<i64>,
        pub description: Option<String>,
    }
}

#[cfg(feature = "http")]
pub(crate) async fn fetch_info(detected: &DetectedGame) -> Result<GameInfo> {
    match detected {
        DetectedGame::Lutris { slug, name, cover, icon, .. } => {
            let client = reqwest::Client::new();
            let response = client.get(format!("https://lutris.net/api/games/{}", slug)).send().await?;
            
            if response.status().is_success() {
                let info: http::Response = serde_json::from_str(&response.text().await?)?;
                Ok(GameInfo {
                    cover: cover.clone(),
                    icon: icon.clone(),
                    name: name.clone(),
                    via_platform: GamePlatform::Lutris,
                    description: info.description.unwrap_or("".to_owned()),
                    developers: vec![],
                    publishers: vec![],
                    app_id: info.steam_id,
                    required_age: None,
                    url: format!("https://lutris.net/games/{}", slug)
                })
            } else {
                Err(anyhow::anyhow!("Failed to fetch game info"))
            }
        },
        _ => unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    #[cfg(feature = "http")]
    async fn test_fetch_info() {
        let info = fetch_info(&DetectedGame::Lutris {
            id: 4168,
            slug: "grand-theft-auto-v".to_string(),
            name: "Grand Theft Auto V".to_string(),
            cover: "https://lutris.net/games/banner/grand-theft-auto-v.jpg".to_string(),
            icon: "https://lutris.net/games/icon/grand-theft-auto-v.png".to_string()
        }).await.unwrap();
        assert_eq!(info.name, "Grand Theft Auto V");
        assert_eq!(info.via_platform, GamePlatform::Lutris);
        assert_eq!(info.app_id, Some(271590));
        assert_eq!(info.required_age, None);
        assert_eq!(info.url, "https://lutris.net/games/grand-theft-auto-v");
        assert_eq!(info.icon, "https://lutris.net/games/icon/grand-theft-auto-v.png".to_string());
        assert_eq!(info.cover, "https://lutris.net/games/banner/grand-theft-auto-v.jpg".to_string());
    }
}