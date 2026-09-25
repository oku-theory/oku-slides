//! Rend un fichier d'options ECharts en SVG : `cargo run --example preview -- options.json 640 400`
use slider_chart::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("chemin du JSON d'options");
    let width = args.next().and_then(|s| s.parse().ok()).unwrap_or(640.0);
    let height = args.next().and_then(|s| s.parse().ok()).unwrap_or(400.0);
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    match parse_option(&json).and_then(|o| ChartsRsRenderer::default().render(&o, Size { width, height })) {
        Ok(svg) => println!("{svg}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
