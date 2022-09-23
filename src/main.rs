extern crate twitchvod;

use std::collections::HashMap;

use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use twitchvod::{config, info};

fn main() -> Result<()> {
    let (url, ex) = match std::env::args().len() {
        2 => (std::env::args().nth(1).unwrap(), config::Executor::Print),
        3 => {
            let (mut e, mut url) = {
                let mut it = std::env::args().skip(1);
                (it.next().unwrap(), it.next().unwrap())
            };

            if url.starts_with("--") {
                std::mem::swap(&mut e, &mut url);
            }

            if e.starts_with("--") {
                e.remove(0);
                e.remove(0);
                let executor = if let Some(executor) =
                    config::load()?.remove(&e).map(config::Executor::Command)
                {
                    executor
                } else {
                    println!("Invalid executor {:?}", e);
                    std::process::exit(1);
                };

                (url, executor)
            } else {
                println!(
                    "USAGE: {} [--<executor>] <URL>",
                    std::env::args().next().unwrap()
                );
                std::process::exit(1);
            }
        }
        _ => {
            println!(
                "USAGE: {} [--<executor>] <URL>",
                std::env::args().next().unwrap()
            );
            std::process::exit(1);
        }
    };

    let info = info::fetch(info::get_video_id(url)?)?;

    let res = {
        let mut keys = Vec::new();
        let mut resolutions = Vec::new();
        for (k, v) in {
            let mut x = info
                .resolutions
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect::<Vec<(&str, &str)>>();
            x.sort_by(|&(r1, _), &(r2, _)| r1.cmp(r2));
            x
        } {
            keys.push(k);
            resolutions.push(v);
        }

        println!("{} - {}", info.channel_name, info.title);

        let res = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select resolution")
            .items(&resolutions[..])
            .default(keys.len() - 1)
            .interact()
            .context("Invalid resolution")?;
        keys[res]
    };

    let mut map = HashMap::new();
    map.insert("url", info.url(res));
    info.into_hashmap_in_place(&mut map);
    ex.execute(&map)
}
