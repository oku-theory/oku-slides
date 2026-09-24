//! `cargo run -p slider-stt --example transcribe -- <fichier.wav> [base|turbo|large] [langue]`
//! Télécharge le modèle s'il manque (dossier partagé avec l'app), puis transcrit.

use std::time::Instant;

use slider_stt::{Model, Stt, resample};

fn main() {
    let mut args = std::env::args().skip(1);
    let wav = args.next().expect("fichier .wav attendu");
    let model = match args.next().as_deref() {
        Some("base") => Model::Base,
        Some("large") => Model::Large,
        _ => Model::Turbo,
    };
    let language = args.next();
    let dir = slider_stt::default_models_dir();

    if !model.is_installed(&dir) {
        eprintln!("téléchargement de {} …", model.label());
        let mut last = 0;
        slider_stt::models::download(model, &dir, |got, total| {
            let pct = got * 100 / total.max(1);
            if pct >= last + 10 {
                last = pct;
                eprintln!("  {pct} %");
            }
            true
        })
        .expect("téléchargement");
    }

    let mut reader = hound::WavReader::open(&wav).expect("wav");
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    let ch = spec.channels as usize;
    let mono: Vec<f32> = samples.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect();
    let audio = resample(&mono, spec.sample_rate, slider_stt::SAMPLE_RATE);

    let stt = Stt::new(&dir);
    stt.configure(slider_stt::Config { model, language });
    let t = Instant::now();
    let text = stt.transcribe_samples(&audio).expect("transcription");
    eprintln!("{:.1} s d'audio transcrites en {:.2} s", audio.len() as f32 / 16000.0, t.elapsed().as_secs_f32());
    println!("{text}");
}
