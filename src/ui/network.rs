use dirs::home_dir;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Error;
use serde_json::json;
use serde_json::Value;
use serde_yaml::to_string;
use std::fs::{self, write};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use std::env;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use crate::config::{self, get_device_name};

#[derive(Serialize, Debug, Deserialize)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
}

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

pub async fn login(
    domain: String,
    username: String,
    password: String,
    port: String,
) -> Result<(), Error> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("X-Emby-Client", HeaderValue::from_static("Tsukimi"));
    headers.insert(
        "X-Emby-Device-Name",
        HeaderValue::from_str(&get_device_name()).unwrap(),
    );
    headers.insert(
        "X-Emby-Device-Id",
        HeaderValue::from_str(&env::var("UUID").unwrap()).unwrap(),
    );
    headers.insert(
        "X-Emby-Client-Version",
        HeaderValue::from_static("0.3.0"),
    );
    headers.insert("X-Emby-Language", HeaderValue::from_static("zh-cn"));

    let body = json!({
        "Username": username,
        "Pw": password
    });

    let res = client
        .post(&format!(
            "{}:{}/emby/Users/authenticatebyname",
            domain, port
        ))
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    let v: Value = res.json().await?;

    // 获取 "User" 对象中的 "Id" 字段
    let user_id = v["User"]["Id"].as_str().unwrap();
    println!("UserId: {}", user_id);

    // 获取 "AccessToken" 字段
    let access_token = v["AccessToken"].as_str().unwrap();
    println!("AccessToken: {}", access_token);

    let config = Config {
        domain,
        username,
        password,
        port,
        user_id: user_id.to_string(),
        access_token: access_token.to_string(),
    };
    let yaml = to_string(&config).unwrap();
    let mut path = home_dir().unwrap();
    path.push(".config");
    path.push("tsukimi.yaml");
    write(path, yaml).unwrap();

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchResult {
    pub Name: String,
    pub Type: String,
    pub Id: String,
    pub UserData: Option<UserData>,
}

struct SearchModel {
    search_results: Vec<SearchResult>,
}

pub(crate) async fn search(searchinfo: String) -> Result<Vec<SearchResult>, Error> {
    let mut model = SearchModel {
        search_results: Vec::new(),
    };
    let server_info = config::set_config();

    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );
    let params = [
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate",
        ),
        ("IncludeItemTypes","Movie,Series"),
        ("StartIndex", "0"),
        ("SortBy", "SortName"),
        ("SortOrder", "Ascending"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("ImageTypeLimit", "1"),
        ("Recursive", "true"),
        ("SearchTerm", &searchinfo),
        ("GroupProgramsBySeries", "true"),
        ("Limit", "50"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let items: Vec<SearchResult> = serde_json::from_value(json["Items"].clone()).unwrap();
    model.search_results = items;
    Ok(model.search_results)
}

#[derive(Deserialize, Debug, Clone)]
pub struct SeriesInfo {
    pub Name: String,
    pub Id: String,
    pub Overview: Option<String>,
    pub IndexNumber: u32,
    pub ParentIndexNumber: u32,
    pub UserData: Option<UserData>,
}



pub async fn get_series_info(id: String) -> Result<Vec<SeriesInfo>, Error> {
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Shows/{}/Episodes",
        server_info.domain, server_info.port, id
    );
    let params = [
        ("Fields", "Overview"),
        ("EnableTotalRecordCount", "true"),
        ("EnableImages", "true"),
        ("UserId", &server_info.user_id),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let seriesinfo: Vec<SeriesInfo> = serde_json::from_value(json["Items"].clone()).unwrap();
    Ok(seriesinfo)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaStream {
    pub DisplayTitle: Option<String>,
    pub Type: String,
    pub DeliveryUrl: Option<String>,
    pub IsExternal: bool,

    pub Title: Option<String>,
    pub DisplayLanguage: Option<String>,

    pub Codec: Option<String>,
    pub BitRate: Option<u64>,
    pub BitDepth: Option<u64>,
    pub AverageFrameRate: Option<f64>,
    pub Height: Option<u64>,
    pub Width: Option<u64>,
    pub PixelFormat: Option<String>,
    pub ColorSpace: Option<String>,

    pub SampleRate: Option<u64>,
    pub Channels: Option<u64>,
    pub ChannelLayout: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaSource {
    pub Id: String,
    pub Name: String,
    pub Size: u64,
    pub Container: String,
    pub DirectStreamUrl: Option<String>,
    pub MediaStreams: Vec<MediaStream>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Media {
    pub MediaSources: Vec<MediaSource>,
    pub PlaySessionId: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub ExternalUrls: Option<Vec<Urls>>,
    pub Overview: Option<String>,
    pub People: Option<Vec<People>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct People {
    pub Name: String,
    pub Id: String,
    pub Role: Option<String>,
    pub Type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Urls {
    pub Name: String,
    pub Url: String,
}

pub async fn get_item_overview(id: String) -> Result<Item, Error> {
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );
    let params = [
        ("Fields", "ShareLevel"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let item: Item = serde_json::from_value(json.clone()).unwrap();
    Ok(item)
}

pub async fn markwatched(id: String, sourceid: String) -> Result<String, Error> {
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/PlayingItems/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );
    println!("{}", url);
    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let inplay = json!({
        "UserId": &server_info.user_id,
        "Id": &id,
        "MediaSourceId": &sourceid,
    });
    let response = client
        .post(&url)
        .query(&params)
        .json(&inplay)
        .send()
        .await?;
    let text = response.text().await?;
    Ok(text)
}

#[derive(Deserialize, Debug, Clone)]
pub struct Resume {
    pub Name: String,
    pub Type: String,
    pub Id: String,
    pub SeriesId: Option<String>,
    pub IndexNumber: Option<u32>,
    pub ParentIndexNumber: Option<u32>,
    pub ParentThumbItemId: Option<String>,
    pub SeriesName: Option<String>,
    pub UserData: Option<UserData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserData {
    pub PlayedPercentage: Option<f64>,
    pub PlaybackPositionTicks: Option<u64>,
    pub Played: bool,
    pub UnplayedItemCount: Option<u32>,
}
struct ResumeModel {
    resume: Vec<Resume>,
}

pub(crate) async fn resume() -> Result<Vec<Resume>, Error> {
    let mut model = ResumeModel { resume: Vec::new() };
    let server_info = config::set_config();

    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/Resume",
        server_info.domain, server_info.port, server_info.user_id
    );
    let params = [
        ("Recursive", "true"),
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
        ),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("ImageTypeLimit", "1"),
        ("MediaTypes", "Video"),
        ("Limit", "8"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let items: Vec<Resume> = serde_json::from_value(json["Items"].clone()).unwrap();
    model.resume = items;
    Ok(model.resume)
}

pub async fn get_image(id: String) -> Result<String, Error> {
    let server_info = config::set_config();

    let result = reqwest::get(&format!(
        "{}:{}/emby/Items/{}/Images/Primary?maxHeight=400",
        server_info.domain, server_info.port, id
    ))
    .await;

    match result {
        Ok(response) => {
            let bytes_result = response.bytes().await;
            match bytes_result {
                Ok(bytes) => {
                    let path_str = format!(
                        "{}/.local/share/tsukimi/",
                        home_dir().expect("msg").display()
                    );
                    let pathbuf = PathBuf::from(path_str);
                    if pathbuf.exists() {
                        fs::write(pathbuf.join(format!("{}.png", id)), &bytes).unwrap();
                    } else {
                        fs::create_dir_all(format!(
                            "{}/.local/share/tsukimi/",
                            home_dir().expect("msg").display()
                        ))
                        .unwrap();

                        fs::write(pathbuf.join(format!("{}.png", id)), &bytes).unwrap();
                    }
                    return Ok(id);
                }
                Err(e) => {
                    eprintln!("loading error");
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            eprintln!("loading error");
            return Err(e.into());
        }
    }
}

pub async fn get_thumbimage(id: String) -> Result<String, Error> {
    let server_info = config::set_config();

    let result = reqwest::get(&format!(
        "{}:{}/emby/Items/{}/Images/Thumb",
        server_info.domain, server_info.port, id
    ))
    .await;

    match result {
        Ok(response) => {
            let bytes_result = response.bytes().await;
            match bytes_result {
                Ok(bytes) => {
                    let path_str = format!(
                        "{}/.local/share/tsukimi/",
                        home_dir().expect("msg").display()
                    );
                    let pathbuf = PathBuf::from(path_str);
                    if pathbuf.exists() {
                        fs::write(pathbuf.join(format!("t{}.png", id)), &bytes).unwrap();
                    } else {
                        fs::create_dir_all(format!(
                            "{}/.local/share/tsukimi/",
                            home_dir().expect("msg").display()
                        ))
                        .unwrap();

                        fs::write(pathbuf.join(format!("t{}.png", id)), &bytes).unwrap();
                    }
                    return Ok(id);
                }
                Err(e) => {
                    eprintln!("loading error");
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            eprintln!("loading error");
            return Err(e.into());
        }
    }
}

pub async fn get_backdropimage(id: String) -> Result<String, Error> {
    let server_info = config::set_config();

    let result = reqwest::get(&format!(
        "{}:{}/emby/Items/{}/Images/Backdrop",
        server_info.domain, server_info.port, id
    ))
    .await;

    match result {
        Ok(response) => {
            let bytes_result = response.bytes().await;
            match bytes_result {
                Ok(bytes) => {
                    let path_str = format!(
                        "{}/.local/share/tsukimi/",
                        home_dir().expect("msg").display()
                    );
                    let pathbuf = PathBuf::from(path_str);
                    if pathbuf.exists() {
                        fs::write(pathbuf.join(format!("b{}.png", id)), &bytes).unwrap();
                    } else {
                        fs::create_dir_all(format!(
                            "{}/.local/share/tsukimi/",
                            home_dir().expect("msg").display()
                        ))
                        .unwrap();

                        fs::write(pathbuf.join(format!("b{}.png", id)), &bytes).unwrap();
                    }
                    return Ok(id);
                }
                Err(e) => {
                    eprintln!("loading error");
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            eprintln!("loading error");
            return Err(e.into());
        }
    }
}

pub async fn get_logoimage(id: String) -> Result<String, Error> {
    let server_info = config::set_config();

    let result = reqwest::get(&format!(
        "{}:{}/emby/Items/{}/Images/Logo",
        server_info.domain, server_info.port, id
    ))
    .await;

    match result {
        Ok(response) => {
            let bytes_result = response.bytes().await;
            match bytes_result {
                Ok(bytes) => {
                    let path_str = format!(
                        "{}/.local/share/tsukimi/",
                        home_dir().expect("msg").display()
                    );
                    let pathbuf = PathBuf::from(path_str);
                    if pathbuf.exists() {
                        fs::write(pathbuf.join(format!("l{}.png", id)), &bytes).unwrap();
                    } else {
                        fs::create_dir_all(format!(
                            "{}/.local/share/tsukimi/",
                            home_dir().expect("msg").display()
                        ))
                        .unwrap();

                        fs::write(pathbuf.join(format!("l{}.png", id)), &bytes).unwrap();
                    }
                    return Ok(id);
                }
                Err(e) => {
                    eprintln!("loading error");
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            eprintln!("loading error");
            return Err(e.into());
        }
    }
}

pub async fn get_mediainfo(id: String) -> Result<Media, Error> {
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );
    let params = [
        ("Fields", "ShareLevel"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json.clone()).unwrap();
    Ok(mediainfo)
}

pub async fn get_playbackinfo(id: String) -> Result<Media, Error> {
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Items/{}/PlaybackInfo",
        server_info.domain, server_info.port, id
    );

    let params = [
        ("StartTimeTicks", "0"),
        ("UserId", &server_info.user_id),
        ("AutoOpenLiveStream", "true"),
        ("IsPlayback", "true"),
        ("AudioStreamIndex", "1"),
        ("SubtitleStreamIndex", "1"),
        ("MaxStreamingBitrate", "4000000"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"DeviceProfile":{"MaxStaticBitrate":140000000,"MaxStreamingBitrate":140000000,"MusicStreamingTranscodingBitrate":192000,"DirectPlayProfiles":[{"Container":"mp4,m4v","Type":"Video","VideoCodec":"h264,av1,vp8,vp9","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Streaming","Protocol":"hls","MaxAudioChannels":"2","MinSegments":"1","BreakOnNonKeyFrames":true},{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"opus","Type":"Audio","AudioCodec":"opus","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"wav","Type":"Audio","AudioCodec":"wav","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"opus","Type":"Audio","AudioCodec":"opus","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"wav","Type":"Audio","AudioCodec":"wav","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"m4s,ts","Type":"Video","AudioCodec":"aac","VideoCodec":"h264","Context":"Streaming","Protocol":"hls","MaxAudioChannels":"2","MinSegments":"1","BreakOnNonKeyFrames":true,"ManifestSubtitles":"vtt"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis","VideoCodec":"vpx","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp4","Type":"Video","AudioCodec":"aac,opus,flac,vorbis","VideoCodec":"h264","Context":"Static","Protocol":"http"}],"ContainerProfiles":[],"CodecProfiles":[{"Type":"VideoAudio","Codec":"aac","Conditions":[{"Condition":"Equals","Property":"IsSecondaryAudio","Value":"false","IsRequired":"false"}]},{"Type":"VideoAudio","Conditions":[{"Condition":"Equals","Property":"IsSecondaryAudio","Value":"false","IsRequired":"false"}]},{"Type":"Video","Codec":"h264","Conditions":[{"Condition":"EqualsAny","Property":"VideoProfile","Value":"high|main|baseline|constrained baseline","IsRequired":false},{"Condition":"LessThanEqual","Property":"VideoLevel","Value":"52","IsRequired":false},{"Condition":"LessThanEqual","Property":"Width","Value":"1280","IsRequired":false}]},{"Type":"Video","Codec":"hevc","Conditions":[{"Condition":"LessThanEqual","Property":"Width","Value":"1280","IsRequired":false}]},{"Type":"Video","Conditions":[{"Condition":"LessThanEqual","Property":"Width","Value":"1280","IsRequired":false}]}],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[{"Type":"Video","Container":"m4v","MimeType":"video/mp4"}]}});
    let response = client
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json.clone()).unwrap();
    return Ok(mediainfo);
}

pub async fn get_sub(id: String,sourceid: String) -> Result<Media, Error> {
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Items/{}/PlaybackInfo",
        server_info.domain, server_info.port, id
    );

    let params = [
        ("StartTimeTicks", "0"),
        ("UserId", &server_info.user_id),
        ("AutoOpenLiveStream", "true"),
        ("IsPlayback", "true"),
        ("AudioStreamIndex", "1"),
        ("SubtitleStreamIndex", "1"),
        ("MediaSourceId", &sourceid),
        ("MaxStreamingBitrate", "4000000"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"DeviceProfile":{"MaxStaticBitrate":140000000,"MaxStreamingBitrate":140000000,"MusicStreamingTranscodingBitrate":192000,"DirectPlayProfiles":[{"Container":"mp4,m4v","Type":"Video","VideoCodec":"h264,av1,vp8,vp9","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Streaming","Protocol":"hls","MaxAudioChannels":"2","MinSegments":"1","BreakOnNonKeyFrames":true},{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"opus","Type":"Audio","AudioCodec":"opus","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"wav","Type":"Audio","AudioCodec":"wav","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"opus","Type":"Audio","AudioCodec":"opus","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"wav","Type":"Audio","AudioCodec":"wav","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"m4s,ts","Type":"Video","AudioCodec":"aac","VideoCodec":"h264","Context":"Streaming","Protocol":"hls","MaxAudioChannels":"2","MinSegments":"1","BreakOnNonKeyFrames":true,"ManifestSubtitles":"vtt"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis","VideoCodec":"vpx","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp4","Type":"Video","AudioCodec":"aac,opus,flac,vorbis","VideoCodec":"h264","Context":"Static","Protocol":"http"}],"ContainerProfiles":[],"CodecProfiles":[{"Type":"VideoAudio","Codec":"aac","Conditions":[{"Condition":"Equals","Property":"IsSecondaryAudio","Value":"false","IsRequired":"false"}]},{"Type":"VideoAudio","Conditions":[{"Condition":"Equals","Property":"IsSecondaryAudio","Value":"false","IsRequired":"false"}]},{"Type":"Video","Codec":"h264","Conditions":[{"Condition":"EqualsAny","Property":"VideoProfile","Value":"high|main|baseline|constrained baseline","IsRequired":false},{"Condition":"LessThanEqual","Property":"VideoLevel","Value":"52","IsRequired":false},{"Condition":"LessThanEqual","Property":"Width","Value":"1280","IsRequired":false}]},{"Type":"Video","Codec":"hevc","Conditions":[{"Condition":"LessThanEqual","Property":"Width","Value":"1280","IsRequired":false}]},{"Type":"Video","Conditions":[{"Condition":"LessThanEqual","Property":"Width","Value":"1280","IsRequired":false}]}],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[{"Type":"Video","Container":"m4v","MimeType":"video/mp4"}]}});
    let response = client
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json.clone()).unwrap();
    return Ok(mediainfo);
}

#[derive(Deserialize, Debug, Clone)]
pub struct View {
    pub Name: String,
    pub Id: String,
    pub CollectionType: Option<String>,
}

pub async fn get_library() -> Result<Vec<View>, Error>{
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Views",
        server_info.domain, server_info.port, server_info.user_id
    );

    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client
        .get(&url)
        .query(&params)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let views: Vec<View> = serde_json::from_value(json["Items"].clone()).unwrap();
    return Ok(views);
}

#[derive(Deserialize, Debug, Clone)]
pub struct Latest {
    pub Name: String,
    pub Id: String,
    pub Type: String,
    pub UserData: Option<UserData>,
    pub ProductionYear: Option<u32>,
}

pub async fn get_latest(id: String,mutex: std::sync::Arc<tokio::sync::Mutex<()>>) -> Result<Vec<Latest>, Error> {
    let _ = mutex.lock().await;
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/Latest",
        server_info.domain, server_info.port, server_info.user_id
    );

    let params = [
        ("Limit", "16"),
        ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear"),
        ("ParentId", &id),
        ("ImageTypeLimit", "1"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client
        .get(&url)
        .query(&params)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let latests: Vec<Latest> = serde_json::from_value(json.clone()).unwrap();
    return Ok(latests);
}

pub async fn get_list(id: String,start: String,mutex: std::sync::Arc<tokio::sync::Mutex<()>>) -> Result<List, Error> {
    let _ = mutex.lock().await;
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );

    let params = [
        ("Limit", "50"),
        ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate"),
        ("ParentId", &id),
        ("ImageTypeLimit", "1"),
        ("StartIndex", &start),
        ("IncludeItemTypes","Movie,Series,MusicAlbum"),
        ("SortBy", "DateCreated,SortName"),
        ("SortOrder", "Descending"),
        ("Recursive", "true"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client
        .get(&url)
        .query(&params)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let latests: List = serde_json::from_value(json.clone()).unwrap();
    return Ok(latests);
}

#[derive(Deserialize, Debug, Clone)]
pub struct List {
    pub TotalRecordCount: u32,
    pub Items: Vec<Latest>,
}

impl Default for List {
    fn default() -> Self {
        List {
            TotalRecordCount: 0,
            Items: Vec::new(),
        }
    }
}

pub struct Back {
    pub id: String,
    pub playsessionid: Option<String>,
    pub mediasourceid: String,
    pub tick: f64,
}

pub async fn positionback(back:Back) {
    let tick = back.tick.to_string();
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Sessions/Playing/Progress",
        server_info.domain, server_info.port
    );

    let params = [
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
    client
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await.unwrap();
}

pub async fn positionstop(back:Back) {
    let tick = back.tick.to_string();
    let server_info = config::set_config();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Sessions/Playing/Stopped",
        server_info.domain, server_info.port
    );

    let params = [
        ("X-Emby-Client-Version", "0.3.0"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
    client
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await.unwrap();
}