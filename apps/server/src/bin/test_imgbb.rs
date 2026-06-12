use ezgif_server::services::video_converter::convert_and_upload_mp4;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _ = dotenvy::dotenv();

    let key = env::var("IMGBB_API_KEY").expect("IMGBB_API_KEY must be set");

    // Tenor MP4 URL
    let sample_url = "https://media.tenor.com/5z2nxEfHHVEAAAPo/virtual-bite.mp4";
    println!("Testing conversion with: {}", sample_url);

    let result = convert_and_upload_mp4(sample_url, &key).await?;
    println!("Success! ImgBB URL: {}", result);

    Ok(())
}
