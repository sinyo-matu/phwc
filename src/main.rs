use chrono::prelude::*;
use headless_chrome::{protocol::page::ScreenshotFormat, Browser, LaunchOptionsBuilder};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path, time::Duration};
use thiserror::Error;
use ureq::get;

mod generate_xlsx;

const ROOT_URL: &str = "https://m.weibo.cn/api/container/getIndex?&containerid=1076037243323531";

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Request(#[from] ureq::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("got some browser error: {0}")]
    Browser(String),
    #[error(transparent)]
    Xlsx(#[from] xlsxwriter::XlsxError),
    #[error(transparent)]
    DatetimeParse(#[from] chrono::ParseError),
}

pub type Result<T> = std::result::Result<T, Error>;
const OUTPUT_DIR: &str = "weibo";

fn main() -> Result<()> {
    set_dir();
    let mut date_map: HashMap<String, u32> = HashMap::new();
    let options = LaunchOptionsBuilder::default()
        .window_size(Some((1280, 800)))
        .build()
        .map_err(Error::Browser)?;
    let browser = Browser::new(options).map_err(|e| Error::Browser(format!("{}", e)))?;
    let root_response = get_root_info()?;
    let vec_mb = root_response
        .data
        .cards
        .iter()
        .map(|c| c.mblog.clone().try_into().unwrap());
    let mut pic_names = Vec::new();
    let vec_card: Vec<RootCard> = root_response
        .data
        .cards
        .iter()
        .map(|c| c.clone().try_into().unwrap())
        .collect();
    for card in vec_card {
        let month = card.mblog.created_at.month();
        let day = card.mblog.created_at.day();
        let q = date_map.entry(format!("{}-{}", month, day)).or_insert(0);
        *q += 1;
        let tab = browser
            .wait_for_initial_tab()
            .map_err(|e| Error::Browser(format!("{}", e)))?;
        println!("正在抓取微博:{}", &card.mblog.id);
        tab.navigate_to(&card.scheme)
            .map_err(|e| Error::Browser(format!("{}", e)))?;
        let wait = 3;
        println!("等待{}秒，让网页完全显示", wait);
        std::thread::sleep(Duration::from_secs(wait));
        let shot = tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .map_err(|e| Error::Browser(format!("{}", e)))?;
        let file_name = format!("{}-{}-{}.png", month, day, q);
        let mut base = Path::new(OUTPUT_DIR).to_path_buf();
        base.push(&file_name);
        fs::write(&base, &shot)?;
        println!("抓取{}成功休息一秒", &card.mblog.id);
        pic_names.push(format!("{}-{}-{}", month, day, q));
        std::thread::sleep(Duration::from_secs(1));
    }
    let now = Local::now();
    let file_name = format!(
        "{}-{}-{}ウェイボー集計.xlsx",
        now.year(),
        now.month(),
        now.day()
    );
    let mut base = Path::new(OUTPUT_DIR).to_path_buf();
    base.push(&file_name);
    let input = vec_mb.zip(pic_names.into_iter()).collect();
    generate_xlsx::generate_xlsx(input, base.as_os_str().to_str().unwrap())?;
    Ok(())
}

fn set_dir() {
    match fs::read_dir(OUTPUT_DIR) {
        Ok(_) => {}
        Err(_) => {
            fs::create_dir(OUTPUT_DIR).unwrap();
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RootResponse {
    data: RootData,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RootData {
    cards: Vec<RootCardRaw>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RootCardRaw {
    scheme: String,
    mblog: MBlogRaw,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RootCard {
    scheme: String,
    mblog: MBlog,
}

impl TryFrom<RootCardRaw> for RootCard {
    type Error = Error;
    fn try_from(r: RootCardRaw) -> Result<Self> {
        Ok(Self {
            scheme: r.scheme,
            mblog: r.mblog.try_into()?,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MBlogRaw {
    text: String,
    id: String,
    reposts_count: u32,
    comments_count: u32,
    reprint_cmt_count: u32,
    attitudes_count: u32,
    created_at: String,
}

impl TryFrom<MBlogRaw> for MBlog {
    type Error = Error;
    fn try_from(r: MBlogRaw) -> Result<Self> {
        let parsed = chrono::DateTime::parse_from_str(&r.created_at, "%a %b %d %T %z %Y")?;
        Ok(Self {
            _text: r.text,
            id: r.id,
            reposts_count: r.reposts_count,
            comments_count: r.comments_count,
            _reprint_cmt_count: r.reprint_cmt_count,
            attitudes_count: r.attitudes_count,
            created_at: parsed,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MBlog {
    _text: String,
    id: String,
    reposts_count: u32,
    comments_count: u32,
    _reprint_cmt_count: u32,
    attitudes_count: u32,
    created_at: DateTime<FixedOffset>,
}

fn get_root_info() -> Result<RootResponse> {
    let res: RootResponse = get(ROOT_URL).call()?.into_json()?;
    Ok(res)
}
