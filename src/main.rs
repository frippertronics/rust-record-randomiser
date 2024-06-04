use image::{load_from_memory_with_format, save_buffer, DynamicImage};
use rand::Rng;
use show_image::ImageInfo;
use std::{io::{self, Read}, path::Path, str::FromStr};
use curl::easy::{Easy, List};

const BASE_DISCOGS_URL: &str = "https://api.discogs.com/releases";
const ARTIST_IDX: usize = 1;
const ALBUM_IDX: usize = 2;
const RELEASE_ID_IDX: usize = 7;
const DISCOGS_TOKEN: &str = "OdBDgCzzXMxliLwtPSoyxWwkWgtxuprBybWIMwiS";
const USER_AGENT: &str = "RustRecordRandomiser/0.1 +http://frippertronics.com";

fn make_request_str(base_url: &str, release_id: &str) -> String {
    format!("{base_url}/{release_id}")
}

#[show_image::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsing CSV-file...");

    let user_cfg = config::Config::builder()
        .add_source(config::File::with_name("config.ini"))
        .build().expect("Please create a config-file from 'config.example.ini!'");
    let mut record_list: Vec<csv::StringRecord> = Vec::new();
    let csv_param = user_cfg.get_string("csv_file")
    .expect("Please specify a CSV-filepath with the 'csv_file'-key!");
    let discogs_user_token = user_cfg.get_string("token")
        .expect("Please specify a Discogs uset-token with the 'token'-key!");
    let csv_path = Path::new(&csv_param).as_os_str();
    println!("{:?}", csv_path);
    let mut csv_reader =  csv::Reader::from_path(csv_path)
        .expect("Invalid CSV-filepath!");
    for result in csv_reader.records() {
        if result.is_err() {
            println!("Skipping invalid line!")
        }
        else {
            record_list.push(result.unwrap());
        }
    }
    let mut rng = rand::thread_rng();
    let random_record = record_list.get(rng.gen_range(0..record_list.len())).unwrap();
    println!("{:?}", random_record);
    // Get album cover
    let artist = random_record.get(ARTIST_IDX).unwrap();
    let album = random_record.get(ALBUM_IDX).unwrap();
    let release_url: String = make_request_str(BASE_DISCOGS_URL, random_record.get(RELEASE_ID_IDX).unwrap());
    println!("{release_url}");
    let mut handle = Easy::new();
    let mut get_buffer: Vec<u8> = Vec::new();
    let mut headers: List = List::new();
    let mut auth_str: String = "Authorization: Discogs token=".to_string();
    auth_str.push_str(&discogs_user_token);
    headers.append(&auth_str).unwrap();
    handle.url(&release_url).unwrap();
    handle.http_headers(headers).unwrap();
    handle.useragent(USER_AGENT).unwrap();
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            get_buffer.extend_from_slice(data);
            Ok(data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }

    let v: serde_json::Value = serde_json::from_slice(&get_buffer).unwrap();
    // TODO: Få ut cover-album fra JSON, last ned og display. Bonus: Reroll-knapp.

    // First image returned is assumed to be the primary image, as specified by the API documentation (if any)
    let primary_image_uri = v["images"][0]["uri"].clone();
    if primary_image_uri.is_string() == false {
        println!("No image! Re-rolling...");
    }
    println!("{:?}",  primary_image_uri);
    let image_url: String = primary_image_uri.as_str().unwrap().to_string();
    handle.url(&image_url)?;
    get_buffer.clear();
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            get_buffer.extend_from_slice(data);
            Ok(data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }

    let image_result: DynamicImage = load_from_memory_with_format(&get_buffer, image::ImageFormat::Jpeg)?;
    let view = show_image::ImageView::new(ImageInfo::rgb8(image_result.width(), image_result.height()), image_result.as_bytes());
    let display_str: String = String::from_str(artist)? + " - " + album;
    let window = show_image::create_window(&display_str, show_image::WindowOptions::default())?;
    window.set_image(&display_str, view)?;
    window.wait_until_destroyed()?;
    Ok(())
}
