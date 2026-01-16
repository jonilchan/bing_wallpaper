#![windows_subsystem = "windows"]

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use chrono::Local;

#[derive(Deserialize, Debug)]
struct BingResponse {
    images: Vec<BingImage>,
}

#[derive(Deserialize, Debug)]
struct BingImage {
    urlbase: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let api_url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1&mkt=zh-CN&uhd=1";
    
    let resp = client.get(api_url).send().await?.json::<BingResponse>().await?;
    if resp.images.is_empty() {
        return Err(anyhow::anyhow!("无法从必应 API 获取图片信息"));
    }

    let image_url = format!("https://www.bing.com{}_UHD.jpg", resp.images[0].urlbase);

    // 1. 获取保存目录（图片目录/BingWallpapers）
    let wallpaper_dir = get_wallpaper_dir()?;
    
    // 2. 生成带日期的文件名 (例如 bing_2023-10-27.jpg)
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let filename = format!("bing_{}.jpg", date_str);
    let save_path = wallpaper_dir.join(&filename);

    // 3. 下载图片 (如果今天还没下载的话)
    if !save_path.exists() {
        download_image(&client, &image_url, &save_path).await?;
    }

    // 4. 设置壁纸
    let path_str = save_path.to_str().ok_or(anyhow::anyhow!("路径转换失败"))?;
    wallpaper::set_from_path(path_str).map_err(|e| anyhow::anyhow!("设置壁纸失败: {}", e))?;
    wallpaper::set_mode(wallpaper::Mode::Crop).ok();

    // 5. 清理旧图片：只保留最近的 7 张
    clean_old_wallpapers(&wallpaper_dir, 7).ok();

    Ok(())
}

fn get_wallpaper_dir() -> Result<PathBuf> {
    let mut path = dirs::picture_dir()
        .or_else(|| dirs::home_dir())
        .ok_or(anyhow::anyhow!("无法找到用户目录"))?;
    
    path.push("BingWallpapers");
    
    if !path.exists() {
        fs::create_dir_all(&path).context("创建壁纸目录失败")?;
    }
    Ok(path)
}

async fn download_image(client: &reqwest::Client, url: &str, path: &PathBuf) -> Result<()> {
    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;
    fs::write(path, bytes).context("写入文件失败")?;
    Ok(())
}

/// 清理目录，只保留最近的 n 个文件
fn clean_old_wallpapers(dir: &PathBuf, keep_count: usize) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|res| res.ok())
        .filter(|e| e.path().is_file())
        .collect();

    // 按修改时间排序（从新到旧）
    entries.sort_by(|a, b| {
        let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });

    // 如果超过 keep_count，删除旧的
    if entries.len() > keep_count {
        for entry in entries.iter().skip(keep_count) {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}