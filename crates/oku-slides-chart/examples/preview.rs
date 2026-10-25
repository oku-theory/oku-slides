//! Rend un fichier d'options ECharts en SVG : `cargo run --example preview -- options.json 640 400`

use oku_slides_chart::{ChartRenderer, ChartsRsRenderer, Result, Size, parse_option};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("chemin du JSON d'options attendu")?;
    let width = args.next().and_then(|s| s.parse().ok()).unwrap_or(640.0);
    let height = args.next().and_then(|s| s.parse().ok()).unwrap_or(400.0);
    let storage = oku_slides_platform::storage::Storage::local()?;
    let json: serde_json::Value = serde_json::from_slice(&storage.read(path.as_ref())?)?;
    let option = parse_option(&json)?;
    let svg = ChartsRsRenderer::default().render(&option, Size { width, height })?;
    println!("{svg}");
    Ok(())
}
