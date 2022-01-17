use chrono::prelude::*;
use headless_chrome::{protocol::page::ScreenshotFormat, Browser, LaunchOptionsBuilder};
use std::{collections::HashMap, fs, path::Path, time::Duration};

use crate::RootCard;
use crate::{Error, Result};

const WAIT: u64 = 3;

pub fn capture_weibos(cards: &[RootCard], out_put_dir: &str) -> Result<()> {
    let mut date_map: HashMap<String, u32> = HashMap::new();
    let options = LaunchOptionsBuilder::default()
        .window_size(Some((1280, 800)))
        .build()
        .map_err(Error::Browser)?;
    let browser = Browser::new(options).map_err(|e| Error::Browser(format!("{}", e)))?;
    for card in cards {
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
        println!("等待{}秒，让网页完全显示", WAIT);
        std::thread::sleep(Duration::from_secs(WAIT));
        let shot = tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .map_err(|e| Error::Browser(format!("{}", e)))?;
        let file_name = format!("{}-{}-{}.png", month, day, q);
        let mut base = Path::new(out_put_dir).to_path_buf();
        base.push(&file_name);
        fs::write(&base, &shot)?;
        println!("抓取{}成功", &card.mblog.id);
    }
    Ok(())
}
