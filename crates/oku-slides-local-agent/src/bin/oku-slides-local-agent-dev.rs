//! Relance de test de l'agent local : `--local-agent` sert ACP, sinon rien.

fn main() {
    oku_slides_local_agent::exit_if_local_agent();
    eprintln!(
        "usage: OKU_SLIDES_LOCAL_AGENT__ENGINE=fake OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT=<file> oku-slides-local-agent-dev --local-agent"
    );
    std::process::exit(2);
}
