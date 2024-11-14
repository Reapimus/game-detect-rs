use std::collections::HashMap;

use anyhow::Result;

mod platforms;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GameInfo {
    pub app_id: Option<i64>, // A steam app id, if it can be mapped to one.
    pub via_platform: GamePlatform,
    pub name: String,
    pub description: String,
    pub cover: String,
    pub icon: String,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub required_age: Option<i32>,
    pub url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GamePlatform {
    #[cfg(feature = "minecraft")]
    MinecraftLauncher,
    #[cfg(feature = "steam")]
    Steam,
    #[cfg(feature = "gamejolt")]
    GameJolt,
    #[cfg(feature = "itchio")]
    ItchIo,
    #[cfg(feature = "lutris")]
    Lutris,
    #[cfg(feature = "roblox")]
    Roblox,
    Custom,
}

#[derive(Debug, Clone, Eq, Hash)]
pub enum DetectedGame {
    #[cfg(feature = "steam")]
    Steam {
        id: i64,
        url: String,
        icon: String,
    },
    #[cfg(feature = "gamejolt")]
    GameJolt {
        id: i64,
        url: String,
        name: String,
        cover: String,
        icon: String,
        developers: Vec<String>,
        publishers: Vec<String>,
    },
    #[cfg(feature = "itchio")]
    ItchIo {
        id: i64,
        url: String,
        name: String,
        description: String,
        cover: String,
        icon: String,
        developers: Vec<String>,
        publishers: Vec<String>,
    },
    #[cfg(feature = "lutris")]
    Lutris {
        id: i64,
        slug: String,
        name: String,
        cover: String,
        icon: String,
    },
    #[cfg(feature = "roblox")]
    Roblox {
        id: i64,
        url: String,
    },
    #[cfg(feature = "minecraft")]
    Minecraft {
        cover: String,
        icon: String,
    },
    #[cfg(feature = "minecraft")]
    MinecraftDungeons {
        cover: String,
        icon: String,
    },
    #[cfg(feature = "minecraft")]
    MinecraftLegends {
        cover: String,
        icon: String,
    },
    Custom(String),
}

#[cfg(feature = "http")]
impl DetectedGame {
    pub async fn get_info(&self) -> Result<GameInfo> {
        match self {
            #[cfg(feature = "steam")]
            DetectedGame::Steam { .. } => platforms::steam::fetch_info(&self).await,
            #[cfg(feature = "gamejolt")]
            DetectedGame::GameJolt { .. } => platforms::gamejolt::fetch_info(&self).await,
            #[cfg(feature = "itchio")]
            DetectedGame::ItchIo { .. } => platforms::itchio::fetch_info(&self).await,
            #[cfg(feature = "lutris")]
            DetectedGame::Lutris { .. } => platforms::lutris::fetch_info(&self).await,
            #[cfg(feature = "roblox")]
            DetectedGame::Roblox { .. } => platforms::roblox::fetch_info(&self).await,
            #[cfg(feature = "minecraft")]
            DetectedGame::Minecraft { .. } => platforms::minecraft::fetch_info(&self).await,
            #[cfg(feature = "minecraft")]
            DetectedGame::MinecraftDungeons { .. } => platforms::minecraft::fetch_info(&self).await,
            #[cfg(feature = "minecraft")]
            DetectedGame::MinecraftLegends { .. } => platforms::minecraft::fetch_info(&self).await,
            DetectedGame::Custom(id) => Ok(GameInfo {
                app_id: None,
                via_platform: GamePlatform::Custom,
                name: id.clone(),
                description: "".to_string(),
                cover: "".to_string(),
                icon: "".to_string(),
                developers: vec![],
                publishers: vec![],
                required_age: None,
                url: "".to_string(),
            }),
        }
    }
}

impl PartialEq for DetectedGame {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DetectedGame::Steam { id: id1, .. }, DetectedGame::Steam { id: id2, .. }) => id1 == id2,
            (DetectedGame::GameJolt { id: id1, .. }, DetectedGame::GameJolt { id: id2, .. }) => id1 == id2,
            (DetectedGame::ItchIo { id: id1, .. }, DetectedGame::ItchIo { id: id2, .. }) => id1 == id2,
            (DetectedGame::Lutris { id: id1, .. }, DetectedGame::Lutris { id: id2, .. }) => id1 == id2,
            (DetectedGame::Roblox { id: id1, .. }, DetectedGame::Roblox { id: id2, .. }) => id1 == id2,
            (DetectedGame::Minecraft { .. }, DetectedGame::Minecraft { .. }) => true,
            (DetectedGame::MinecraftDungeons { .. }, DetectedGame::MinecraftDungeons { .. }) => true,
            (DetectedGame::MinecraftLegends { .. }, DetectedGame::MinecraftLegends { .. }) => true,
            _ => false,
        }
    }
}

pub fn detect_game(custom_games: Option<HashMap<String, String>>) -> Result<Option<DetectedGame>> {
    let processes = procfs::process::all_processes().unwrap();
    let processes: Vec<procfs::process::Process> = processes.filter_map(|process| process.ok()).collect();

    if custom_games.is_some() {
        for (id, name) in custom_games.unwrap() {
            for process in &processes {
                let cmd = process.cmdline().ok();
                match cmd {
                    Some(cmd) => {
                        let cmd = cmd.join(" ");
                        if cmd.to_lowercase().contains(&name.to_lowercase()) {
                            return Ok(Some(DetectedGame::Custom(id)));
                        }
                    }
                    None => continue,
                }
            }
        }
    }
    
    #[cfg(feature = "steam")]
    if let Some(game) = platforms::steam::detect_game()? {
        return Ok(Some(game));
    }

    #[cfg(feature = "itchio")]
    if let Some(game) = platforms::itchio::detect_game(&processes)? {
        return Ok(Some(game));
    }

    #[cfg(feature = "gamejolt")]
    if let Some(game) = platforms::gamejolt::detect_game(&processes)? {
        return Ok(Some(game));
    }

    #[cfg(all(feature = "lutris", target_os = "linux"))]
    if let Some(game) = platforms::lutris::detect_game(&processes)? {
        return Ok(Some(game));
    }

    #[cfg(feature = "roblox")]
    if let Some(game) = platforms::roblox::detect_game(&processes)? {
        return Ok(Some(game));
    }

    #[cfg(feature = "minecraft")]
    if let Some(game) = platforms::minecraft::detect_game(&processes)? {
        return Ok(Some(game));
    }

    Ok(None)
}
