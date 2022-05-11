use crate::Result;
use chrono::{Datelike, Timelike};
use chrono_tz::Asia::Shanghai;
use xlsxwriter::Workbook;

use crate::MBlog;

const SHEET_HEADERS: &[(&str, f64)] = &[
    ("投稿時間", 8.0),
    ("配信リンク", 8.0),
    ("備考", 8.0),
    ("リーチ数(PV)", 8.0),
    ("リツート数", 4.0),
    ("コメント数", 3.0),
    ("いい数", 3.0),
];
const CHARACTER_WIDTH: f64 = 3.0;
pub fn generate_xlsx(input: Vec<(MBlog, String)>, file_name: &str) -> Result<()> {
    let workbook = Workbook::new(file_name);
    let general_format = workbook
        .add_format()
        .set_border(xlsxwriter::FormatBorder::Thin)
        .set_align(xlsxwriter::FormatAlignment::Center);
    let text_format = workbook
        .add_format()
        .set_text_wrap()
        .set_border(xlsxwriter::FormatBorder::Thin)
        .set_align(xlsxwriter::FormatAlignment::Center);
    let mut sheet = workbook.add_worksheet(None)?;
    for (index, (header, len)) in SHEET_HEADERS.iter().enumerate() {
        let header_chars_q = header.chars().fold(0.0, |acc, _c| acc + 1.0);
        let column_len = f64::max(header_chars_q, *len);
        sheet.set_column(
            index as u16,
            index as u16,
            column_len * CHARACTER_WIDTH,
            None,
        )?;
        sheet.write_string(0, index as u16, header, Some(&general_format))?;
    }
    for (row, (mb, pic_name)) in input.iter().enumerate() {
        let row = row + 1;
        let datetime = mb.created_at.with_timezone(&Shanghai);
        let time = format!(
            "{}年{}月{}日{}時{}分",
            datetime.year(),
            datetime.month(),
            datetime.day(),
            datetime.hour(),
            datetime.minute(),
        );
        let cols = vec![
            time,
            String::from(pic_name),
            String::from(""),
            String::from(""),
            format!("{}", mb.reposts_count),
            format!("{}", mb.comments_count),
            format!("{}", mb.attitudes_count),
        ];
        for (col, content) in cols.iter().enumerate() {
            if col == 1 {
                sheet.write_string(row as u32, col as u16, content, Some(&text_format))?
            }
            sheet.write_string(row as u32, col as u16, content, Some(&general_format))?
        }
    }
    workbook.close()?;
    Ok(())
}
