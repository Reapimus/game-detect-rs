#[cfg(feature = "http")]
use crate::{GameInfo, GamePlatform};
use crate::DetectedGame;
use anyhow::Result;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs::{exists, read_to_string};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use home::home_dir;
#[cfg(target_family = "windows")]
use registry::{Hive, Security, Data};

#[cfg(target_family = "windows")]
const REG_TREE_PATH: &str = r"Software\Valve\Steam";

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod vdf {
    use std::collections::HashMap;
    use serde::Deserialize;
    use anyhow::Result;

    #[derive(Deserialize)]
    pub enum VDFValue {
        Map(HashMap<String, VDFValue>),
        Number(i64),
    }

    impl VDFValue {
        pub fn get(&self, key: &str) -> Result<&VDFValue> {
            match self {
                VDFValue::Map(map) => {
                    if let Some(value) = map.get(key) {
                        Ok(value)
                    } else {
                        Err(anyhow::anyhow!("Key not found"))
                    }
                }
                _ => Err(anyhow::anyhow!("Key is not a map")),
            }
        }
    }
}

pub fn detect_game() -> Result<Option<DetectedGame>> {
    #[cfg(target_family = "windows")]
    {
        let regkey = Hive::CurrentUser.open(REG_TREE_PATH, Security::Read)?;
        let val = regkey.value("/v/RunningAppId")?;
        return match val {
            Data::U32(appid) => Ok(Some(DetectedGame::Steam {
                id: appid as i64,
                url: format!("https://store.steampowered.com/app/{0}", appid),
                icon: format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{0}/hero_capsule.jpg", appid)
            })),
            Data::U64(appid) => Ok(Some(DetectedGame::Steam {
                id: appid as i64,
                url: format!("https://store.steampowered.com/app/{0}", appid),
                icon: format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{0}/hero_capsule.jpg", appid)
            })),
            _ => Ok(None)
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        #[cfg(target_os = "linux")]
        let steam_path = home_dir().unwrap().join(".steam");
        #[cfg(target_os = "macos")]
        let steam_path = home_dir().unwrap().join("Library/Application Support/Steam");
        if !exists(steam_path.join("registry.vdf"))? {
            return Ok(None);
        }
        let reg = read_to_string(steam_path.join("registry.vdf"))?;
        let parsed: vdf::VDFValue = vdf_reader::from_str(&reg)?;
        let current_app = parsed.get("Registry")?
            .get("HKCU")?
            .get("Software")?
            .get("Valve")?
            .get("Steam")?
            .get("RunningAppID")?;

        return match current_app {
            vdf::VDFValue::Number(appid) => Ok(Some(DetectedGame::Steam {
                id: *appid,
                url: format!("https://store.steampowered.com/app/{0}", appid),
                icon: format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{0}/hero_capsule.jpg", appid)
            })),
            _ => Ok(None)
        }
    }
}

#[cfg(feature = "http")]
mod http {
    use std::collections::HashMap;

    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(crate) struct App {
        pub name: String,
        pub short_description: String,
        pub required_age: String,
        pub developers: Vec<String>,
        pub publishers: Vec<String>,
        pub background_raw: String
    }

    #[derive(Deserialize)]
    pub(crate) struct ResponseInner {
        pub data: App
    }

    pub(crate) type Response = HashMap<i32, ResponseInner>;
}

#[cfg(feature = "http")]
pub(crate) async fn fetch_info(detected: &DetectedGame) -> Result<GameInfo> {
    match detected {
        DetectedGame::Steam { id, icon, url } => {
            let client = reqwest::Client::new();
            let response = client.get(format!("https://store.steampowered.com/api/appdetails?appids={0}", id)).send().await?;

            if response.status().is_success() {
                let response: http::Response = serde_json::from_str(&response.text().await?)?;
                let app_id = *id as i32;
                let app = &response.get(&app_id).ok_or(anyhow::anyhow!("App not found"))?.data;
                Ok(GameInfo {
                    cover: app.background_raw.clone(),
                    icon: icon.clone(),
                    name: app.name.clone(),
                    via_platform: GamePlatform::Steam,
                    description: app.short_description.clone(),
                    developers: app.developers.clone(),
                    publishers: app.publishers.clone(),
                    app_id: Some(id.clone()),
                    required_age: app.required_age.parse::<i32>().ok(),
                    url: url.clone()
                })
            } else {
                Err(anyhow::anyhow!("Failed to fetch game info"))
            }
        }
        _ => Err(anyhow::anyhow!("Not a steam game"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    #[cfg(feature = "http")]
    async fn test_fetch_info() {
        let info = fetch_info(&DetectedGame::Steam {
            id: 601050,
            url: "https://store.steampowered.com/app/601050".to_string(),
            icon: "https://cdn.cloudflare.steamstatic.com/steam/apps/601050/hero_capsule.jpg".to_string()
        }).await.unwrap();
        assert_eq!(info.name, "Attack on Titan 2 - A.O.T.2");
        assert_eq!(info.via_platform, GamePlatform::Steam);
        assert_eq!(info.app_id, Some(601050));
        assert_eq!(info.required_age, Some(15));
        assert_eq!(info.url, "https://store.steampowered.com/app/601050");
        assert_eq!(info.icon, "https://cdn.cloudflare.steamstatic.com/steam/apps/601050/hero_capsule.jpg".to_string());
        // Omit the query parameters to guarantee the cover URL is static
        assert_eq!(info.cover.split("?").collect::<Vec<&str>>()[0].to_owned(), "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/601050/page.bg.jpg".to_string());
        assert_eq!(info.developers, vec!["KOEI TECMO GAMES CO., LTD.".to_owned()]);
        assert_eq!(info.publishers, vec!["KOEI TECMO GAMES CO., LTD.".to_owned()]);
        assert_eq!(info.description, "Abandon all fear. Attack on Titan 2 is the gripping sequel to the action game based on the worldwide hit anime series &quot;Attack on Titan.&quot;".to_string());
    }
}