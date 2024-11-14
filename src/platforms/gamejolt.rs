#[cfg(feature = "http")]
use crate::{GameInfo, GamePlatform};
use std::{collections::HashMap, fs::{exists, read_to_string}};
use anyhow::Result;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use home::home_dir;
#[cfg(target_family = "windows")]
use dirs::data_local_dir;
use serde::Deserialize;

use crate::DetectedGame;

#[derive(Deserialize)]
struct Package {
    pub game_id: i64,
    pub running_pid: Option<String>
}

#[derive(Deserialize)]
struct Packages {
    pub objects: HashMap<i64, Package>
}

#[derive(Deserialize)]
struct Developer {
    pub display_name: String
}

#[derive(Deserialize)]
struct MediaItem {
    pub img_url: String
}

#[derive(Deserialize)]
struct Game {
    pub title: String,
    pub developer: Developer,
    pub slug: Option<String>,
    pub header_media_item: MediaItem,
    pub thumbnail_media_item: MediaItem
}

#[derive(Deserialize)]
struct Games {
    pub objects: HashMap<i64, Game>
}

pub fn detect_game(processes: &Vec<procfs::process::Process>) -> Result<Option<DetectedGame>> {
    #[cfg(target_family = "windows")]
    let data_dir = data_local_dir().unwrap().join("game-jolt-client").join("Default");
    #[cfg(target_os = "linux")]
    let data_dir = home_dir().unwrap().join(".config").join("game-jolt-client").join("Default");
    #[cfg(target_os = "macos")]
    let data_dir = home_dir().unwrap().join("Library").join("Application Support").join("game-jolt-client").join("Default");

    if !exists(&data_dir).unwrap_or(false) {
        return Ok(None);
    }

    let packages: Packages = serde_json::from_str(&read_to_string(&data_dir.join("packages.wttf"))?)?;
    let games: Games = serde_json::from_str(&read_to_string(&data_dir.join("games.wttf"))?)?;

    for (_id, package) in packages.objects {
        if !&package.running_pid.is_some() {
            continue;
        }
        for process in processes {
            if &process.pid == &package.running_pid.as_ref().unwrap()[2..].parse::<i32>()? {
                let game_id = package.game_id;
                let game_details = match games.objects.get(&game_id) {
                    Some(game) => game,
                    None => continue
                };

                return Ok(Some(DetectedGame::GameJolt {
                    id: game_id.clone(),
                    name: game_details.title.clone(),
                    url: format!("https://gamejolt.com/games/{0}/{1}", game_details.slug.clone().unwrap_or("redirect".to_owned()), game_id),
                    developers: vec![game_details.developer.display_name.clone()],
                    publishers: vec![game_details.developer.display_name.clone()],
                    icon: game_details.header_media_item.img_url.clone(),
                    cover: game_details.thumbnail_media_item.img_url.clone()
                }))
            }
        }
    }
    Ok(None)
}

#[cfg(feature = "http")]
mod http {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(crate) struct Developer {
        pub display_name: Option<String>,
        pub name: String
    }

    #[derive(Deserialize)]
    pub(crate) struct MediaItem {
        pub img_url: String
    }

    #[derive(Deserialize)]
    pub(crate) struct LinkAttrs {
        pub href: String,
        #[allow(unused)]
        pub title: Option<String>,
        #[allow(unused)]
        pub autolink: Option<bool>
    }

    #[derive(Deserialize)]
    pub(crate) struct TagAttrs {
        #[allow(unused)]
        pub tag: String,
    }

    #[derive(Deserialize)]
    #[serde(tag = "type")]
    pub(crate) enum MarkdownMark {
        #[serde(rename = "link")]
        Link {
            attrs: LinkAttrs
        },
        #[serde(rename = "strong")]
        Bold,
        #[serde(rename = "em")]
        Italic,
        #[serde(rename = "tag")]
        Tag {
            #[allow(unused)]
            attrs: TagAttrs
        }
    }

    #[derive(Deserialize)]
    #[serde(tag = "type")]
    pub(crate) enum MarkdownTag {
        #[serde(rename = "paragraph")]
        Paragraph {
            content: Vec<MarkdownTag>
        },
        #[serde(rename = "text")]
        Text {
            marks: Option<Vec<MarkdownMark>>,
            text: String
        },
        #[serde(rename = "hardBreak")]
        HardBreak,
        #[serde(rename = "bulletList")]
        BulletList {
            content: Vec<MarkdownTag>
        },
        #[serde(rename = "listItem")]
        ListItem {
            content: Vec<MarkdownTag>
        },
    }

    impl ToString for MarkdownTag {
        fn to_string(&self) -> String {
            match self {
                MarkdownTag::Paragraph { content } => {
                    content.iter().map(|tag| tag.to_string()).collect::<Vec<String>>().join("") + "\n"
                }
                MarkdownTag::Text { marks, text } => {
                    let mut text = text.clone();
                    if let Some(marks) = marks {
                        for mark in marks {
                            match mark {
                                MarkdownMark::Link { attrs } => {
                                    text = format!("[{}]({})", text, attrs.href);
                                }
                                MarkdownMark::Bold => {
                                    text = format!("**{}**", text);
                                }
                                MarkdownMark::Italic => {
                                    text = format!("*{}*", text);
                                }
                                MarkdownMark::Tag { .. } => {
                                    text = text;
                                }
                            }
                        }
                    }
                    text
                }
                MarkdownTag::HardBreak => "\n".to_owned(),
                MarkdownTag::BulletList { content } => {
                    content.iter().map(|tag| "    ".to_string() + &tag.to_string()).collect::<Vec<String>>().join("\n").to_owned()
                }
                MarkdownTag::ListItem { content } => {
                    "• ".to_owned() + &content.iter().map(|tag| tag.to_string()).collect::<Vec<String>>().join("\n")
                }
            }
        }
    }

    #[derive(Deserialize)]
    pub(crate) struct Description {
        pub content: Vec<MarkdownTag>
    }

    impl ToString for Description {
        fn to_string(&self) -> String {
            self.content.iter().map(|tag| tag.to_string()).collect::<Vec<String>>().join("\n")
        }
    }

    #[derive(Deserialize)]
    pub(crate) struct Game {
        pub has_adult_content: bool,
        pub developer: Developer,
        pub title: String,
        pub header_media_item: MediaItem,
        pub thumbnail_media_item: MediaItem,
        pub description_content: String,
        pub slug: Option<String>
    }

    #[derive(Deserialize)]
    pub(crate) struct ResponseInner {
        pub game: Game
    }

    #[derive(Deserialize)]
    pub(crate) struct Response {
        pub payload: ResponseInner
    }
}

#[cfg(feature = "http")]
pub(crate) async fn fetch_info(detected: &DetectedGame) -> Result<GameInfo> {
    match detected {
        DetectedGame::GameJolt { id, .. } => {
            let client = reqwest::Client::new();
            let response = client.get(format!("https://gamejolt.com/site-api/web/discover/games/{}", id)).send().await?;

            if response.status().is_success() {
                let response: http::Response = serde_json::from_str(&response.text().await?)?;
                let game = response.payload.game;
                let age = match game.has_adult_content {
                    true => 18,
                    false => 13
                };

                let description_content: http::Description = serde_json::from_str(&game.description_content)?;
                let description = description_content.to_string();
                Ok(GameInfo {
                    cover: game.thumbnail_media_item.img_url,
                    icon: game.header_media_item.img_url,
                    name: game.title,
                    via_platform: GamePlatform::GameJolt,
                    description,
                    developers: vec![game.developer.display_name.to_owned().unwrap_or(game.developer.name.to_owned())],
                    publishers: vec![game.developer.display_name.to_owned().unwrap_or(game.developer.name.to_owned())],
                    app_id: None,
                    required_age: Some(age),
                    url: format!("https://gamejolt.com/games/{0}/{1}", game.slug.unwrap_or("redirect".to_owned()), id)
                })
            } else {
                return Err(anyhow::anyhow!("Failed to fetch game info"));
            }
        }
        _ => unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    #[cfg(feature = "http")]
    async fn test_fetch_info() {
        let info = fetch_info(&DetectedGame::GameJolt {
            id: 303626,
            name: "Pixel Heist".to_owned(),
            url: "https://gamejolt.com/games/Pixel_Heist/303626".to_owned(),
            developers: vec!["REALyeswecamp".to_owned()],
            publishers: vec!["REALyeswecamp".to_owned()],
            icon: "https://i.gjcdn.net/public-data/games/4/126/303626/backgroundpixelheistbanner-2smwc36v.png".to_owned(),
            cover: "https://i.gjcdn.net/public-data/games/4/126/303626/screenshot02-1920x1080-fe105cac968c5f05ea91be18a7888cfc-dxd2t2jh.png".to_owned()
        }).await.unwrap();
        assert_eq!(info.name, "Pixel Heist");
        assert_eq!(info.developers, vec!["REALyeswecamp".to_owned()]);
        assert_eq!(info.publishers, vec!["REALyeswecamp".to_owned()]);
        assert_eq!(info.required_age, Some(13));
        assert_eq!(info.url, "https://gamejolt.com/games/Pixel_Heist/303626");
        assert_eq!(info.via_platform, GamePlatform::GameJolt);
        assert_eq!(info.icon, "https://i.gjcdn.net/public-data/games/4/126/303626/backgroundpixelheistbanner-2smwc36v.png".to_owned());
        assert_eq!(info.cover, "https://i.gjcdn.net/public-data/games/4/126/303626/screenshot02-1920x1080-fe105cac968c5f05ea91be18a7888cfc-dxd2t2jh.png".to_owned());
    }
}