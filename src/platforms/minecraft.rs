#[cfg(feature = "http")]
use crate::{GameInfo, GamePlatform};
use crate::DetectedGame;
use anyhow::Result;

pub fn detect_game(processes: &Vec<procfs::process::Process>) -> Result<Option<DetectedGame>> {
    for process in processes {
        let cmd = process.cmdline().ok();
        match cmd {
            Some(cmd) => {
                let cmd = cmd.join(" ").to_lowercase();
                if cmd.contains("minecraft") {
                    if cmd.contains("legends") {
                        return Ok(Some(DetectedGame::MinecraftLegends {
                            cover: "https://www.minecraft.net/content/dam/minecraft/home/home-hero-1200x600.jpg".to_owned(),
                            icon: "https://www.minecraft.net/etc.clientlibs/minecraft/clientlibs/main/resources/favicon-96x96.png".to_owned()
                        }));
                    } else if cmd.contains("dungeons") {
                        return Ok(Some(DetectedGame::MinecraftDungeons {
                            cover: "https://www.minecraft.net/content/dam/minecraft/home/home-hero-1200x600.jpg".to_owned(),
                            icon: "https://www.minecraft.net/etc.clientlibs/minecraft/clientlibs/main/resources/favicon-96x96.png".to_owned()
                        }));
                    } else {
                        return Ok(Some(DetectedGame::Minecraft {
                            cover: "https://www.minecraft.net/content/dam/minecraft/home/home-hero-1200x600.jpg".to_owned(),
                            icon: "https://www.minecraft.net/etc.clientlibs/minecraft/clientlibs/main/resources/favicon-96x96.png".to_owned()
                        }));
                    }
                }
            }
            None => continue,
        }
    }
    Ok(None)
}

#[cfg(feature = "http")]
pub(crate) async fn fetch_info(detected: &DetectedGame) -> Result<GameInfo> {
    match detected {
        DetectedGame::Minecraft { cover, icon } =>
            Ok(GameInfo {
                cover: cover.to_owned(),
                icon: icon.to_owned(),
                name: "Minecraft".to_owned(),
                via_platform: GamePlatform::MinecraftLauncher,
                description: "".to_owned(),
                developers: vec!["Mojang Studios".to_owned()],
                publishers: vec!["Mojang Studios".to_owned()],
                app_id: None,
                required_age: Some(10),
                url: "https://www.xbox.com/en-US/games/store/-/9NXP44L49SHJ".to_owned()
            }),
        DetectedGame::MinecraftLegends { cover, icon } =>
            Ok(GameInfo {
                cover: cover.to_owned(),
                icon: icon.to_owned(),
                name: "Minecraft Legends".to_owned(),
                via_platform: GamePlatform::MinecraftLauncher,
                description: "Discover the mysteries of Minecraft Legends, a new action strategy game. Explore a gentle land of rich resources and lush biomes on the brink of destruction. The ravaging piglins have arrived, and it’s up to you to inspire your allies and lead them in strategic battles to save the Overworld!".to_owned(),
                developers: vec!["Mojang Studios".to_owned()],
                publishers: vec!["Mojang Studios".to_owned()],
                app_id: Some(1928870),
                required_age: Some(10),
                url: "https://store.steampowered.com/app/1928870".to_owned()
            }),
        DetectedGame::MinecraftDungeons { cover, icon } =>
            Ok(GameInfo {
                cover: cover.to_owned(),
                icon: icon.to_owned(),
                name: "Minecraft Dungeons".to_owned(),
                via_platform: GamePlatform::MinecraftLauncher,
                description: "Fight your way through an exciting action-adventure game, inspired by classic dungeon crawlers and set in the Minecraft universe!".to_owned(),
                developers: vec!["Mojang Studios".to_owned()],
                publishers: vec!["Mojang Studios".to_owned()],
                app_id: Some(1672970),
                required_age: Some(10),
                url: "https://store.steampowered.com/app/1672970".to_owned()
            }),
        _ => unreachable!()
    }
}