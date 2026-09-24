//! Compare les moteurs de dictée sur un même fichier WAV.
//!
//! `cargo run --release -p slider-stt --features ct2 --example bench -- <fichier.wav> [turbo|large|base] [langue] [essais]`
//!
//! whisper.cpp (Metal sur Mac avec `--features metal`) contre CTranslate2, le
//! moteur de faster-whisper (processeur ; `--features ct2-accelerate` sur Mac).
//! Chaque moteur transcrit le fichier plusieurs fois ; on garde la médiane.

use std::time::Instant;

use slider_stt::engine::{Engine, WhisperCpp};
use slider_stt::{Model, clean, resample};

fn main() {
    let mut args = std::env::args().skip(1);
    let wav = args.next().expect("fichier .wav attendu");
    let model = match args.next().as_deref() {
        Some("base") => Model::Base,
        Some("large") => Model::Large,
        _ => Model::Turbo,
    };
    let language = args.next().filter(|l| l != "auto");
    let runs: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(3).max(1);
    let dir = slider_stt::default_models_dir();
    let audio = read_wav(&wav);
    let seconds = audio.len() as f64 / slider_stt::SAMPLE_RATE as f64;
    eprintln!(
        "{wav} : {seconds:.1} s d'audio · modèle {} · langue {} · {runs} essais\n",
        model.label(),
        language.as_deref().unwrap_or("auto")
    );

    let mut rows = Vec::new();
    // `ECHO_BENCH=ct2` ou `ECHO_BENCH=cpp` : un seul moteur.
    let only = std::env::var("ECHO_BENCH").unwrap_or_default();

    // whisper.cpp
    if only != "ct2" {
        if !model.is_installed(&dir) {
            eprintln!("téléchargement du modèle ggml…");
            slider_stt::models::download(model, &dir, |_, _| true).expect("téléchargement");
        }
        let t = Instant::now();
        let cpp = WhisperCpp::load(&model.path(&dir)).expect("chargement");
        rows.push(measure(
            &cpp,
            format!("whisper.cpp ({})", model.file_name()),
            t.elapsed().as_secs_f64(),
            &audio,
            language.as_deref(),
            runs,
        ));
    }

    // CTranslate2
    #[cfg(feature = "ct2")]
    if only != "cpp" {
        use slider_stt::engine::Ct2;
        let mut shown = u64::MAX;
        let ct2_dir = slider_stt::models::download_ct2(model, &dir, |file, got, total| {
            // Sans taille annoncée, on compte en Mo.
            let pct = (got * 100).checked_div(total);
            let step = pct.map_or(got >> 24, |p| p / 10);
            if step != shown {
                shown = step;
                match pct {
                    Some(p) => eprint!("\rtéléchargement CTranslate2 : {file} {p} %   "),
                    None => eprint!("\rtéléchargement CTranslate2 : {file} {} Mo   ", got >> 20),
                }
            }
        })
        .expect("téléchargement CTranslate2");
        eprintln!();
        for (label, int8, beam) in [("int8, glouton", true, 1), ("int8, faisceau 5", true, 5)] {
            let t = Instant::now();
            let ct2 = Ct2::load(&ct2_dir, int8, beam).expect("chargement CTranslate2");
            rows.push(measure(
                &ct2,
                format!("CTranslate2 ({label})"),
                t.elapsed().as_secs_f64(),
                &audio,
                language.as_deref(),
                runs,
            ));
        }
    }
    #[cfg(not(feature = "ct2"))]
    eprintln!("(CTranslate2 non compilé : ajoutez --features ct2)");

    println!("\n| Moteur | Chargement | Transcription (médiane) | × temps réel |");
    println!("| --- | --- | --- | --- |");
    for r in &rows {
        println!("| {} | {:.2} s | {:.2} s | {:.1}× |", r.name, r.load, r.median, seconds / r.median);
    }
    println!();
    for r in &rows {
        println!("{} :\n  {}\n", r.name, r.text);
    }
}

struct Row {
    name: String,
    load: f64,
    median: f64,
    text: String,
}

fn measure(engine: &dyn Engine, name: String, load: f64, audio: &[f32], language: Option<&str>, runs: usize) -> Row {
    // Un tour à blanc : caches et noyaux GPU prêts.
    let text = engine.transcribe(audio, language, None, false).expect("transcription");
    let mut times: Vec<f64> = (0..runs)
        .map(|_| {
            let t = Instant::now();
            engine.transcribe(audio, language, None, false).expect("transcription");
            t.elapsed().as_secs_f64()
        })
        .collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[times.len() / 2];
    eprintln!("{name} : {median:.2} s ({} essais)", times.len());
    Row { name, load, median, text: clean(&text) }
}

fn read_wav(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("wav");
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    let ch = spec.channels as usize;
    let mono: Vec<f32> = samples.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect();
    resample(&mono, spec.sample_rate, slider_stt::SAMPLE_RATE)
}
