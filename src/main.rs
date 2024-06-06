use curl::easy;
use image::{load_from_memory_with_format, DynamicImage};
use rand::Rng;
use show_image::event;
use std::{fs::File, path::Path, str::FromStr};

const BASE_DISCOGS_URL: &str = "https://api.discogs.com/releases";
const ARTIST_IDX: usize = 1;
const ALBUM_IDX: usize = 2;
const RELEASE_ID_IDX: usize = 7;
const USER_AGENT: &str = "RustRecordRandomiser/0.1 +https://frippertronics.com";

fn get_random_record_from_csv(csv_reader: &mut csv::Reader<File>) -> csv::StringRecord {
    let line_count = csv_reader.records().count();
    csv_reader
        .seek(csv::Position::new())
        .expect("Failed resetting CSV reader position");
    let mut rng = rand::thread_rng();
    let random_number = rng.gen_range(1..line_count);

    let mut record_count = 0;
    for result in csv_reader.records() {
        if result.is_err() {
            println!("Skipping invalid line!")
        } else if record_count == random_number {
            return result.unwrap();
        } else {
            record_count += 1;
        }
    }
    return csv::StringRecord::new();
}

fn download_record_image(release_id: &str, discogs_user_token: &str) -> DynamicImage {
    let mut handle = easy::Easy::new();
    let mut get_buffer: Vec<u8> = Vec::new();
    let auth_str: String = "Authorization: Discogs token=".to_string() + &discogs_user_token;
    let mut headers: easy::List = easy::List::new();

    // 1. Request the release info to get the cover image URL
    let release_url = BASE_DISCOGS_URL.to_string() + "/" + release_id;
    headers.append(&auth_str).unwrap();
    handle.url(&release_url).unwrap();
    handle.http_headers(headers).unwrap();
    handle.useragent(USER_AGENT).unwrap();
    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|data| {
                get_buffer.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }

    // 2. Get first image returned is assumed to be the primary image, as specified by the API documentation (if any)
    let release_resp: serde_json::Value = serde_json::from_slice(&get_buffer).unwrap();
    let primary_image_uri = release_resp["images"][0]["uri"]
        .as_str()
        .expect("Expected image URI to be a string!")
        .to_string();
    handle.url(&primary_image_uri).expect("Invalid image URL");
    get_buffer.clear();
    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|data| {
                get_buffer.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }

    return load_from_memory_with_format(&get_buffer, image::ImageFormat::Jpeg)
        .expect("Invalid image data!");
}

#[show_image::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_cfg = config::Config::builder()
        .add_source(config::File::with_name("config.ini"))
        .build()
        .expect("Please create a config-file from 'config.example.ini!'");
    let csv_param = user_cfg
        .get_string("csv_file")
        .expect("Please specify a CSV-filepath with the 'csv_file'-key!");
    let discogs_user_token = user_cfg
        .get_string("token")
        .expect("Please specify a Discogs user-token with the 'token'-key!");

    let csv_path = Path::new(&csv_param).as_os_str();
    let mut csv_reader = csv::Reader::from_path(csv_path).expect("Invalid CSV-filepath!");
    'program: loop {
        let random_record = get_random_record_from_csv(&mut csv_reader);

        let artist = random_record.get(ARTIST_IDX).unwrap();
        let album = random_record.get(ALBUM_IDX).unwrap();
        let release_id = random_record.get(RELEASE_ID_IDX).unwrap();
        let record_image = download_record_image(&release_id, &discogs_user_token);

        let view = show_image::ImageView::new(
            show_image::ImageInfo::rgb8(record_image.width(), record_image.height()),
            record_image.as_bytes(),
        );
        let display_str: String = String::from_str(artist)? + " - " + album;
        let window = show_image::create_window(&display_str, show_image::WindowOptions::default())?;
        window.set_image(&display_str, view)?;

        'rand_loop: for window_event in window.event_channel()? {
            match window_event {
                event::WindowEvent::MouseButton(mouse_event) => {
                    if (mouse_event.button == event::MouseButton::Left)
                        && (mouse_event.state == event::ElementState::Pressed)
                    {
                        break 'rand_loop;
                    }
                }
                event::WindowEvent::CloseRequested(close_event) => {
                    if close_event.window_id == window.id() {
                        break 'program;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
