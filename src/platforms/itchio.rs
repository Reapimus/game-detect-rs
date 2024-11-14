#[cfg(feature = "http")]
use crate::{GameInfo, GamePlatform};
use std::{fs::{exists, read_to_string}, io::Read};
use anyhow::Result;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use home::home_dir;
#[cfg(target_family = "windows")]
use dirs::data_local_dir;
use serde::Deserialize;
use regex::Regex;

use crate::DetectedGame;

#[derive(Deserialize)]
struct PartialPreferences {
    #[serde(rename = "installLocations")]
    pub install_locations: Vec<String>,
}

#[derive(Deserialize)]
struct ReceiptUser {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Deserialize)]
struct ReceiptInner {
    pub id: i64,
    pub title: String,
    pub url: String,
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    #[serde(rename = "shortText")]
    pub description: String,
    pub user: ReceiptUser,
}

#[derive(Deserialize)]
struct GameReceipt {
    pub game: ReceiptInner
}

fn escape_str(s: &str) -> String {
    let re = Regex::new(r"[.*+?^${}()|[\]\\]").unwrap();
    re.replace_all(s, "\\$0").to_string()
}

pub fn detect_game(processes: &Vec<procfs::process::Process>) -> Result<Option<DetectedGame>> {
    let mut install_locations: Vec<String> = vec![];
    #[cfg(target_family = "windows")]
    let preferences_path = data_local_dir().unwrap().join("itch").join("preferences.json");
    #[cfg(target_os = "linux")]
    let preferences_path = home_dir().unwrap().join(".config").join("itch").join("preferences.json");
    #[cfg(target_os = "macos")]
    let preferences_path = home_dir().unwrap().join("Library").join("Application Support").join("itch").join("preferences.json");

    #[cfg(target_family = "windows")]
    if exists("C://Games/Itch Games").unwrap_or(false) {
        install_locations.push("C://Games/Itch Games".to_owned());
    }
    #[cfg(target_os = "linux")]
    if exists(home_dir().unwrap().join(".config").join("itch").join("apps")).unwrap_or(false) {
        install_locations.push(home_dir().unwrap().join(".config").join("itch").join("apps").to_str().unwrap().to_owned());
    }
    #[cfg(target_os = "macos")]
    if exists(home_dir().unwrap().join("Library").join("Application Support").join("itch").join("apps")).unwrap_or(false) {
        install_locations.push(home_dir().unwrap().join("Library").join("Application Support").join("itch").join("apps").to_str().unwrap().to_owned());
    }

    if exists(&preferences_path).unwrap_or(false) {
        let preferences: PartialPreferences = serde_json::from_str(&read_to_string(&preferences_path)?)?;
        for location in preferences.install_locations {
            if exists(&location).unwrap_or(false) {
                install_locations.push(location);
            }
        }
    }

    for location in install_locations {
        for process in processes {
            let cmd = process.cmdline().ok();
            match cmd {
                Some(cmd) => {
                    let cmd = cmd.join(" ").to_lowercase();
                    if cmd.contains(&location.to_lowercase()) {
                        let re = Regex::new(&format!(r"({0}/[a-zA-z0-9\-\_ &]+)/", escape_str(&cmd))).unwrap();
                        let game_base_folder = match re.captures(&cmd) {
                            Some(captures) => Some(captures.get(1).unwrap().as_str().to_owned()),
                            None => None,
                        };

                        if let Some(game_base_folder) = game_base_folder {
                            let game_receipt = read_to_string(format!("{}/.itch/receipt.json.gz", game_base_folder))?;
                            let mut unpacked_receipt = String::new();
                            let mut decoder = flate2::read::GzDecoder::new(game_receipt.as_bytes());
                            decoder.read_to_string(&mut unpacked_receipt)?;
                            let game_receipt: GameReceipt = serde_json::from_str(&unpacked_receipt)?;

                            return Ok(Some(DetectedGame::ItchIo {
                                id: game_receipt.game.id,
                                name: game_receipt.game.title,
                                url: game_receipt.game.url,
                                cover: game_receipt.game.cover_url.clone(),
                                icon: game_receipt.game.cover_url.clone(),
                                description: game_receipt.game.description,
                                developers: vec![game_receipt.game.user.display_name.clone()],
                                publishers: vec![game_receipt.game.user.display_name.clone()],
                            }))
                        }
                    }
                }
                None => continue,
            }
        }
    }
    Ok(None)
}

#[cfg(feature = "http")]
pub(crate) async fn fetch_info(detected: &DetectedGame) -> Result<GameInfo> {
    match detected {
        DetectedGame::ItchIo { url, name, description, cover, icon, developers, publishers, .. } => {
            Ok(GameInfo {
                cover: cover.clone(),
                icon: icon.clone(),
                name: name.clone(),
                via_platform: GamePlatform::ItchIo,
                description: description.clone(),
                developers: developers.clone(),
                publishers: publishers.clone(),
                app_id: None,
                required_age: None,
                url: url.clone(),
            })
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    #[cfg(feature = "http")]
    async fn test_fetch_info() {
        let detected = DetectedGame::ItchIo {
            id: 2513640,
            name: "Ignited Entry".to_owned(),
            url: "https://jordiboi.itch.io/ignited-entry".to_owned(),
            cover: "https://img.itch.zone//aW1nLzE4NTA1Mjc3LnBuZw==//315x250%23c//dw6M7j.png".to_owned(),
            icon: "https://img.itch.zone//aW1nLzE4NTA1Mjc3LnBuZw==//315x250%23c//dw6M7j.png".to_owned(),
            description: "".to_owned(),
            developers: vec!["jordiboi".to_owned()],
            publishers: vec!["jordiboi".to_owned()],
        };
        let info = fetch_info(&detected).await.unwrap();
        assert_eq!(info.name, "Ignited Entry");
        assert_eq!(info.url, "https://jordiboi.itch.io/ignited-entry");
        assert_eq!(info.cover, "https://img.itch.zone//aW1nLzE4NTA1Mjc3LnBuZw==//315x250%23c//dw6M7j.png");
        assert_eq!(info.icon, "https://img.itch.zone//aW1nLzE4NTA1Mjc3LnBuZw==//315x250%23c//dw6M7j.png");
        assert_eq!(info.description, "");
        assert_eq!(info.developers, vec!["jordiboi"]);
        assert_eq!(info.publishers, vec!["jordiboi"]);
    }
}