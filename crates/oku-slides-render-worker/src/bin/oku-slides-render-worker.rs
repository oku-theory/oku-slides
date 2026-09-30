//! Binaire de test : le travailleur seul (`--render-worker`).

fn main() {
    oku_slides_render_worker::exit_if_worker();
    eprintln!(
        "usage: oku-slides-render-worker --render-worker --max-frame-bytes <n> --timeout-ms <n> --memory-bytes <n>"
    );
    std::process::exit(2);
}
