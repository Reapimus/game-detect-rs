#[cfg(feature = "minecraft")]
pub mod minecraft;
#[cfg(feature = "roblox")]
pub mod roblox;
#[cfg(feature = "steam")]
pub mod steam;
#[cfg(all(feature = "lutris", target_os = "linux"))]
pub mod lutris;
#[cfg(feature = "itchio")]
pub mod itchio;
#[cfg(feature = "gamejolt")]
pub mod gamejolt;