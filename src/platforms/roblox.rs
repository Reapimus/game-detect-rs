#[cfg(feature = "http")]
use crate::{GameInfo, GamePlatform};
#[cfg(target_family = "windows")]
use std::fs::{read_dir, read_to_string};
use anyhow::Result;
use regex::Regex;

use crate::DetectedGame;

pub fn detect_game(processes: &Vec<procfs::process::Process>) -> Result<Option<DetectedGame>> {
    for process in processes {
        let cmd = process.cmdline().ok();
        match cmd {
            Some(cmd) => {
                let cmd = cmd.join(" ").to_lowercase();
                if cmd.contains("robloxplayerbeta") {
                    #[cfg(target_family = "windows")]
                    {
                        let log_dir = dirs::data_local_dir().unwrap().join("Roblox\\logs");
                        for entry in read_dir(log_dir).unwrap() {
                            let entry = entry.unwrap();
                            let path = entry.path();
                            if path.is_file() {
                                if path.extension().unwrap() == "log" {
                                    let logs = read_to_string(path).unwrap();
                                    let log = logs.split("\n").collect::<Vec<&str>>();
                                    let logs_reversed = log.iter().rev().collect::<Vec<&&str>>();

                                    let mut place_id = None;
                                    let mut disconnect_found = false;

                                    if logs_reversed.len() > 0 {
                                        for line in logs_reversed {
                                            let line = *line;

                                            if place_id.is_some() || disconnect_found {
                                                return Ok(Some(DetectedGame::Roblox {
                                                    id: place_id.unwrap(),
                                                    url: format!("https://roblox.com/games/{0}", place_id.unwrap())
                                                }))
                                            }

                                            if line.contains("[FLog::Network] Time to disconnect replication data:") {
                                                disconnect_found = true;
                                            }

                                            if line.contains("Report game_join_loadtime: placeid:") {
                                                let split = line.split("placeid:").collect::<Vec<&str>>();
                                                if split.len() != 2 {
                                                    continue;
                                                }
                                                let place_id_line = split[1].trim().parse::<i64>();
                                                match place_id_line {
                                                    Ok(place_id_line) => {
                                                        place_id = Some(place_id_line);
                                                    }
                                                    Err(_) => continue,
                                                }
                                            }
                                        }
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    let re = Regex::new(r"placeId=(\d+)").unwrap();
                    let caps = re.captures(&cmd);
                    match caps {
                        Some(caps) => {
                            let place_id = caps.get(1).unwrap().as_str().parse::<i64>().unwrap();
                            return Ok(Some(DetectedGame::Roblox {
                                id: place_id,
                                url: format!("https://roblox.com/games/{0}", place_id)
                            }))
                        }
                        None => continue,
                    }
                }
            }
            None => continue,
        }
    }
    Ok(None)
}

#[cfg(feature = "http")]
mod http {
    use serde::Deserialize;
    
    #[derive(Deserialize, Clone)]
    pub(crate) struct UniverseResponse {
        #[serde(rename = "universeId")]
        pub universe_id: i64
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct MediaAsset {
        #[serde(rename = "imageUrl")]
        pub image_url: String
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct IconResponse {
        pub data: Vec<MediaAsset>
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct ThumbnailResponseData {
        pub thumbnails: Vec<MediaAsset>
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct ThumbnailResponseInner {
        pub data: ThumbnailResponseData
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct ThumbnailResponse {
        pub data: ThumbnailResponseInner
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct Developer {
        pub name: String
    }
    
    #[derive(Deserialize, Clone)]
    pub(crate) struct Game {
        pub name: String,
        pub description: String,
        pub creator: Developer,
        #[serde(rename = "rootPlaceId")]
        pub root_place_id: i64
    }

    #[derive(Deserialize, Clone)]
    pub(crate) struct GameResponse {
        pub data: Vec<Game>
    }
}

#[cfg(feature = "http")]
pub(crate) async fn fetch_info(detected: &DetectedGame) -> Result<GameInfo> {
    match detected {
        DetectedGame::Roblox { id, .. } => {
            let client = reqwest::Client::new();
            let universe_response = client.get(format!("https://apis.roblox.com/universes/v1/places/{}/universe", id)).send().await?;
            
            if universe_response.status().is_success() {
                let universe_response: http::UniverseResponse = serde_json::from_str(&universe_response.text().await?).unwrap();
                let universe_id = universe_response.universe_id;

                let icon_response = client.get(format!("https://thumbnails.roblox.com/v1/games/icons?universeIds={}&size=50x50&format=png", universe_id)).send().await?;
                let icon_response: http::IconResponse = serde_json::from_str(&icon_response.text().await?).unwrap();
                let icon = icon_response.data[0].image_url.clone();

                let thumbnail_response = client.get(format!("https://thumbnails.roblox.com/v1/games/multiget/thumbnails?universeIds={}&size=768x432&format=png&countPerUniverse=1", universe_id)).send().await?;
                let thumbnail_response: http::ThumbnailResponse = serde_json::from_str(&thumbnail_response.text().await?).unwrap();
                let thumbnail = thumbnail_response.data.data.thumbnails[0].image_url.clone();

                let game_response = client.get(format!("https://games.roblox.com/v1/games?universeIds={}", universe_id)).send().await?;
                let game_response: http::GameResponse = serde_json::from_str(&game_response.text().await?).unwrap();
                let game = game_response.data[0].clone();

                return Ok(GameInfo {
                    app_id: None,
                    name: game.name,
                    description: game.description,
                    icon,
                    cover: thumbnail,
                    developers: vec![game.creator.name.to_owned()],
                    publishers: vec![game.creator.name.to_owned()],
                    via_platform: GamePlatform::Roblox,
                    required_age: None,
                    url: format!("https://roblox.com/games/{}", game.root_place_id)
                });
            } else {
                return Err(anyhow::anyhow!("Failed to fetch game info"));
            }
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
        let info = fetch_info(&DetectedGame::Roblox {
            id: 1818,
            url: "https://roblox.com/games/1818".to_string(),
        }).await.unwrap();
        // We don't compare icon & cover because those aren't guaranteed to be static.
        assert_eq!(info.name, "Classic: Crossroads");
        assert_eq!(info.description, "The classic ROBLOX level is back!");
        assert_eq!(info.developers, vec!["Roblox"]);
        assert_eq!(info.publishers, vec!["Roblox"]);
        assert_eq!(info.via_platform, GamePlatform::Roblox);
        assert_eq!(info.required_age, None);
        assert_eq!(info.url, "https://roblox.com/games/1818");
    }
}