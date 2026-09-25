//! Rend un fichier d'options ECharts en SVG : `cargo run --example preview -- options.json 640 400`

use slider_chart::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("chemin du JSON d'options attendu")?;
    let width = args.next().and_then(|s| s.parse().ok()).unwrap_or(640.0);
    let height = args.next().and_then(|s| s.parse().ok()).unwrap_or(400.0);
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let option = parse_option(&json)?;
    let svg = ChartsRsRenderer::default().render(&option, Size { width, height })?;
    println!("{svg}");
    Ok(())
}
