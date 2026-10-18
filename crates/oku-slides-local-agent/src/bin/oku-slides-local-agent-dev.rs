//! Relance de test de l'agent local : `--local-agent` sert ACP, sinon rien.

fn main() {
    oku_slides_local_agent::exit_if_local_agent();
    eprintln!("usage: oku-slides-local-agent-dev --local-agent --engine fake --script <file>");
    std::process::exit(2);
}
