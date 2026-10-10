pub mod gog;
pub mod legendary;
pub mod nile;
pub mod sideload;

use std::collections::{HashMap, HashSet};

use crate::{path::CommonPath, prelude::StrictPath};

use crate::{
    resource::{config::root, manifest::Os},
    scan::{TitleFinder, launchers::LauncherGame, semantic::Prefix},
};

mod games_config {
    use std::collections::HashMap;

    pub fn path(id: &str) -> String {
        format!("GamesConfig/{id}.json")
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Data(pub HashMap<String, Game>);

    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Game {
        #[serde(rename_all = "camelCase")]
        Config {
            wine_prefix: String,
            wine_version: Wine,
        },
        IgnoreOther(serde::de::IgnoredAny),
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Wine {
        #[serde(rename = "type")]
        pub wine_type: String,
    }
}

pub fn scan(
    root: &root::Heroic,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, HashSet<LauncherGame>> {
    let mut games = HashMap::<String, HashSet<LauncherGame>>::new();

    for (title, info) in legendary::scan(root, title_finder, legendary) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in gog::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in nile::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in sideload::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    if Os::HOST == Os::Linux
        && let Some(normal_home) = CommonPath::Home.get().map(StrictPath::new)
        && let Some(flatpak_home) = root.flatpak_home()
    {
        log::debug!("For Flatpak root {root:?}, translating home {normal_home:?} to {flatpak_home:?}");
        return games
            .into_iter()
            .map(|(title, mut info)| {
                let flatpak: HashSet<_> = info
                    .iter()
                    .map(|info| info.replace_in_paths(&normal_home, &flatpak_home))
                    .collect();

                info.extend(flatpak);

                (title, info)
            })
            .collect();
    }

    games
}

fn find_prefix(
    heroic_path: &StrictPath,
    game_name: &str,
    platform: Option<&str>,
    app_name: &str,
) -> Option<StrictPath> {
    log::trace!(
        "Will try to find prefix for Heroic game: {} (app={}, platform={:?})",
        game_name,
        app_name,
        platform
    );

    let games_config_path = heroic_path.joined(games_config::path(app_name));

    let content = match games_config_path.try_read() {
        Ok(content) => content,
        Err(e) => {
            log::trace!("Failed to read {:?}: {}", &games_config_path, e);
            return None;
        }
    };

    match serde_json::from_str::<games_config::Data>(&content) {
        Ok(games_config_wrapper) => {
            let game_config = games_config_wrapper.0.get(app_name)?;

            match game_config {
                games_config::Game::Config {
                    wine_version,
                    wine_prefix,
                } => match wine_version.wine_type.as_str() {
                    "wine" => {
                        log::trace!(
                            "Found Heroic Wine prefix for {} ({}) -> adding {}",
                            game_name,
                            app_name,
                            wine_prefix
                        );
                        Some(StrictPath::new(wine_prefix.clone()))
                    }

                    "proton" => {
                        // This would usually be `{wine_prefix}/pfx`,
                        // but in at least one case in a Flatpak install,
                        // that was a broken link and the real prefix was just `wine_prefix`.
                        // When `pfx` is valid, we use only that, since a `drive_c` beside it
                        // would be left over from running the game without Proton.
                        // If `pfx` is a link back to `wine_prefix` (e.g., with umu), then we keep `wine_prefix`,
                        // since found files are recorded by their real path.
                        let pfx = StrictPath::new(wine_prefix.clone()).joined("pfx");
                        let pfx_is_link = std::path::Path::new(wine_prefix).join("pfx").is_symlink();
                        let prefix = if !pfx_is_link && Prefix::validated(&pfx).is_some() {
                            pfx
                        } else {
                            StrictPath::new(wine_prefix.clone())
                        };
                        log::trace!(
                            "Found Heroic Proton prefix for {} ({}), adding {:?}",
                            game_name,
                            app_name,
                            prefix
                        );
                        Some(prefix)
                    }

                    _ => {
                        log::info!(
                            "Found Heroic Windows game {} ({}) with unknown wine_type: {:#?}",
                            game_name,
                            app_name,
                            wine_version.wine_type
                        );
                        None
                    }
                },
                games_config::Game::IgnoreOther(_) => None,
            }
        }
        Err(e) => {
            log::trace!("Failed to parse {:?}: {}", &games_config_path, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heroic_with_proton_prefix(heroic: &std::path::Path, prefix: &std::path::Path) {
        std::fs::create_dir_all(heroic.join("GamesConfig")).unwrap();
        std::fs::write(
            heroic.join("GamesConfig/app.json"),
            serde_json::json!({
                "app": {
                    "winePrefix": prefix.to_string_lossy(),
                    "wineVersion": { "type": "proton" },
                },
            })
            .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn proton_prefix_uses_pfx_when_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("prefix");
        heroic_with_proton_prefix(tmp.path(), &prefix);
        std::fs::create_dir_all(prefix.join("drive_c/users/deck")).unwrap();
        std::fs::create_dir_all(prefix.join("pfx/drive_c/users/steamuser")).unwrap();
        std::fs::File::create(prefix.join("pfx/system.reg")).unwrap();

        assert_eq!(
            Some(StrictPath::new(prefix.to_string_lossy().to_string()).joined("pfx")),
            find_prefix(&StrictPath::from(tmp.path()), "game", None, "app"),
        );
    }

    #[test]
    fn proton_prefix_falls_back_without_valid_pfx() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("prefix");
        heroic_with_proton_prefix(tmp.path(), &prefix);
        std::fs::create_dir_all(prefix.join("pfx")).unwrap();

        assert_eq!(
            Some(StrictPath::new(prefix.to_string_lossy().to_string())),
            find_prefix(&StrictPath::from(tmp.path()), "game", None, "app"),
        );
    }

    #[cfg(unix)]
    #[test]
    fn proton_prefix_ignores_pfx_link_to_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path().join("prefix");
        heroic_with_proton_prefix(tmp.path(), &prefix);
        std::fs::create_dir_all(prefix.join("drive_c/users/steamuser")).unwrap();
        std::fs::File::create(prefix.join("system.reg")).unwrap();
        std::os::unix::fs::symlink(&prefix, prefix.join("pfx")).unwrap();

        assert_eq!(
            Some(StrictPath::new(prefix.to_string_lossy().to_string())),
            find_prefix(&StrictPath::from(tmp.path()), "game", None, "app"),
        );
    }
}
