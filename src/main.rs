use chrono::prelude::*;
use chrono_tz::Asia::Shanghai;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fs, ops::Sub, path::Path};
use thiserror::Error;
use ureq::get;

struct Config {
    output_dir: String,
    recent: i64,
    accent: i64,
    limit: usize,
    captures: bool,
}

static CONFIG: Lazy<Config> = Lazy::new(|| {
    let now = Local::now();
    let output_dir = format!("{}年{}月{}日收集weibo", now.year(), now.month(), now.day());
    let captures = dotenv::var("CAPTURES").unwrap().parse::<bool>().unwrap();
    let recent = dotenv::var("RECENT").unwrap().parse::<i64>().unwrap();
    let accent = dotenv::var("ACCENT").unwrap().parse::<i64>().unwrap();
    let limit = dotenv::var("LIMIT").unwrap().parse::<usize>().unwrap();
    Config {
        output_dir,
        recent,
        accent,
        limit,
        captures,
    }
});

mod captures;
mod generate_xlsx;
use captures::capture_weibos;

const ROOT_URL: &str =
    "https://m.weibo.cn/api/container/getIndex?&containerid=1076037243323531&page=";

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
    #[error(transparent)]
    ParseBool(#[from] core::str::ParseBoolError),
    #[error(transparent)]
    DotEnv(#[from] dotenv::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    dotenv::dotenv()?;
    set_dir();
    let mut date_map: HashMap<String, u32> = HashMap::new();
    let cards = get_root_info()?;
    let mut pic_names = Vec::new();
    for card in cards.iter() {
        let month = card.mblog.created_at.month();
        let day = card.mblog.created_at.day();
        let q = date_map.entry(format!("{}-{}", month, day)).or_insert(0);
        *q += 1;
        pic_names.push(format!("{}-{}-{}", month, day, q));
    }
    if CONFIG.captures {
        println!("因截图功能被启用，将进行截图。请再等等...");
        capture_weibos(&cards, CONFIG.output_dir.as_str())?;
    }
    let now = Local::now();
    let file_name = format!(
        "{}-{}-{}ウェイボー集計.xlsx",
        now.year(),
        now.month(),
        now.day()
    );
    let mut base = Path::new(CONFIG.output_dir.as_str()).to_path_buf();
    base.push(&file_name);
    let mblogs = cards.into_iter().map(|c| c.mblog);
    let input = mblogs.zip(pic_names.into_iter()).collect();
    generate_xlsx::generate_xlsx(input, base.as_os_str().to_str().unwrap())?;
    println!("微博收集结束！");
    Ok(())
}

fn set_dir() {
    match fs::read_dir(CONFIG.output_dir.as_str()) {
        Ok(_) => {}
        Err(_) => {
            fs::create_dir(CONFIG.output_dir.as_str()).unwrap();
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
    retweeted_status: Option<Value>,
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
            retweeted_status: r.retweeted_status,
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
    retweeted_status: Option<Value>,
}

fn get_root_info() -> Result<Vec<RootCard>> {
    let mut cards: Vec<RootCard> = Vec::new();
    let recent_boundary = Local::now()
        .with_timezone(&Shanghai)
        .sub(chrono::Duration::days(CONFIG.recent));
    let accent_boundary = recent_boundary.sub(chrono::Duration::days(CONFIG.accent));
    println!(
        "将收集{}至{}的微博,请稍等...",
        recent_boundary, accent_boundary
    );
    'outer: for page in 1.. {
        let res: RootResponse = get(format!("{}{}", ROOT_URL, page).as_str())
            .call()?
            .into_json()?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        for card_raw in res.data.cards {
            let card: RootCard = card_raw.try_into()?;
            let is_boundary = card.mblog.created_at.with_timezone(&Shanghai) <= accent_boundary;
            if card.mblog.created_at.with_timezone(&Shanghai) <= recent_boundary
                && card.mblog.retweeted_status.is_none()
            {
                cards.push(card)
            }
            if is_boundary || cards.len() >= CONFIG.limit {
                break 'outer;
            }
        }
    }
    println!("收集了{}条微博", cards.len());
    Ok(cards)
}
