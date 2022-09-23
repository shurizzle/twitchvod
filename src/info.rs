use std::{borrow::Borrow, collections::HashMap};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use url::Url;

fn parse_url<S: Borrow<str>>(url: S) -> Result<Url> {
    Url::parse(url.borrow()).context("Invalid url")
}

fn is_twitch(url: &Url) -> bool {
    if let Some(h) = url.host() {
        let h = h.to_string();
        let h = if let Some(x) = h.strip_prefix("www.") {
            x
        } else {
            &h[..]
        };
        h == "twitch.tv"
    } else {
        false
    }
}

fn extract_video_id(url: &Url) -> Option<&str> {
    let mut res = None;

    if let Some(segs) = url.path_segments() {
        for (i, s) in segs.enumerate() {
            match i {
                0 => {
                    if s != "videos" {
                        return None;
                    }
                }
                1 => {
                    if matches!(s.parse::<usize>(), Ok(_)) {
                        res = Some(s);
                    }
                }
                _ => {
                    return None;
                }
            }
        }
    }
    res
}

pub fn get_video_id<S: Borrow<str>>(url: S) -> Result<String> {
    let url = parse_url(url)?;

    if is_twitch(&url) {
        if let Some(id) = extract_video_id(&url).map(|id| id.to_string()) {
            Ok(id)
        } else {
            Err(anyhow!("Not a video url"))
        }
    } else {
        Err(anyhow!("Not a twitch url"))
    }
}

#[derive(Debug, Deserialize)]
struct RawChannel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawVideoInfo {
    title: String,
    animated_preview_url: String,
    channel: RawChannel,
    resolutions: HashMap<String, String>,
    broadcast_type: String,
}

#[derive(Debug)]
pub struct VideoInfo {
    pub title: String,
    pub domain: String,
    pub special_id: String,
    pub id: String,
    pub channel_name: String,
    pub resolutions: HashMap<String, String>,
    pub broadcast_type: String,
}

impl VideoInfo {
    fn from_raw(value: RawVideoInfo, id: String) -> Result<Self> {
        let RawVideoInfo {
            title,
            animated_preview_url,
            resolutions,
            broadcast_type,
            channel,
        } = value;

        let url = parse_url(animated_preview_url)?;
        let special_id = if let Some(segs) = url.path_segments() {
            if let Some(special_id) = segs.take_while(|x| !x.contains("storyboards")).last() {
                special_id.to_string()
            } else {
                return Err(anyhow!("Invalid data received"));
            }
        } else {
            return Err(anyhow!("Invalid data received"));
        };

        let domain = if let Some(d) = url.domain() {
            d.to_string()
        } else {
            return Err(anyhow!("Invalid data received"));
        };

        Ok(VideoInfo {
            title,
            domain,
            special_id,
            id,
            resolutions,
            channel_name: channel.name,
            broadcast_type,
        })
    }

    pub fn url<S: AsRef<str>>(&self, res: S) -> String {
        match self.broadcast_type.as_str() {
            "highlight" => format!(
                "https://{}/{}/{}/highlight-{}.m3u8",
                self.domain,
                self.special_id,
                res.as_ref(),
                self.id
            ),
            "upload" => format!(
                "https://{}/{}/{}/{}/{}/index-dvr.m3u8",
                self.domain,
                self.channel_name,
                self.id,
                self.special_id,
                res.as_ref()
            ),
            _ => format!(
                "https://{}/{}/{}/index-dvr.m3u8",
                self.domain,
                self.special_id,
                res.as_ref()
            ),
        }
    }

    pub fn into_hashmap_in_place(self, map: &mut HashMap<&'static str, String>) {
        map.insert("title", self.title);
        map.insert("domain", self.domain);
        map.insert("special_id", self.special_id);
        map.insert("id", self.id);
        map.insert("channel_name", self.channel_name);
        map.insert("broadcast_type", self.broadcast_type);
    }

    pub fn into_hashmap(self) -> HashMap<&'static str, String> {
        let mut res = HashMap::new();
        self.into_hashmap_in_place(&mut res);
        res
    }
}

pub fn fetch(id: String) -> Result<VideoInfo> {
    VideoInfo::from_raw(
        serde_json::from_str(
            ureq::get(&format!("https://api.twitch.tv/kraken/videos/{id}"))
                .set("Client-Id", "kimne78kx3ncx6brgo4mv6wki5h1ko")
                .set("Accept", "application/vnd.twitchtv.v5+json")
                .call()
                .context("Failed to fetch data")?
                .into_string()
                .context("Invalid data received")?
                .as_str(),
        )
        .context("Invalid data received")?,
        id,
    )
}
