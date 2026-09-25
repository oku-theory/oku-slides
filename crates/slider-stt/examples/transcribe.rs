//! `cargo run --release -p slider-stt --example transcribe -- <fichier.wav> [base|turbo|large] [langue]`
//! Télécharge le modèle s'il manque (dans le dossier des modèles de l'app,
//! voir les réglages), découpe l'audio en phrases (détection de la voix) et
//! les transcrit comme une dictée.

use std::time::Instant;

use slider_stt::{Model, Stt, resample};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let wav = args.next().ok_or("fichier .wav attendu")?;
    let model = match args.next().as_deref() {
        Some("base") => Model::Base,
        Some("large") => Model::Large,
        _ => Model::Turbo,
    };
    let language = args.next();
    let settings = slider_platform::config::Settings::load()?;
    let dir = settings.paths.models();

    if !model.is_installed(&dir) {
        eprintln!("téléchargement de {} …", model.label());
        let mut last = 0;
        slider_stt::download(model, &dir, &settings.stt, |got, total| {
            let pct = got * 100 / total.max(1);
            if pct >= last + 10 {
                last = pct;
                eprintln!("  {pct} %");
            }
            true
        })?;
    }

    let mut reader = hound::WavReader::open(&wav)?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            reader.samples::<i16>().map(|s| s.map(|s| f32::from(s) / 32768.0)).collect::<Result<_, _>>()?
        }
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
    };
    let ch = spec.channels as usize;
    let mono: Vec<f32> = samples.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect();
    let audio = resample(&mono, spec.sample_rate, slider_stt::SAMPLE_RATE);

    let stt = Stt::new(&dir, settings.stt.clone());
    // Les réglages de l'app sont remis en place à la fin.
    let before = stt.status().config;
    stt.configure(slider_stt::Config { model, language, ..before.clone() });
    let t = Instant::now();
    let load = stt.transcribe_samples(&[]).map(|_| t.elapsed().as_secs_f32());
    let t = Instant::now();
    let result = stt.transcribe_samples(&audio);
    let took = t.elapsed().as_secs_f32();
    stt.configure(before);
    let (text, phrases) = result?;
    let secs = audio.len() as f32 / 16000.0;
    let spoken: f32 = phrases.iter().map(|(a, b, _)| b - a).sum();
    eprintln!(
        "chargement {:.2} s · {secs:.1} s d'audio dont {spoken:.1} s de parole ({} phrases) · transcrit en {took:.2} s ({:.1}× temps réel)",
        load.unwrap_or(0.0),
        phrases.len(),
        secs / took.max(0.001),
    );
    for (a, b, t) in &phrases {
        eprintln!("  [{a:5.1} → {b:5.1}] {t}");
    }
    println!("{text}");
    Ok(())
}
