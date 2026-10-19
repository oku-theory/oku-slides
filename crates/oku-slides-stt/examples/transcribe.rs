//! `cargo run --release -p oku-slides-stt --example transcribe -- <fichier.wav> [base|turbo|large|apple] [langue]`
//! Télécharge le modèle s'il manque (dans le dossier des modèles de l'app,
//! voir les réglages), découpe l'audio en phrases (détection de la voix) et
//! les transcrit comme une dictée.

use std::time::Instant;

use oku_slides_stt::{Model, Stt, resample};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let wav = args.next().ok_or("fichier .wav attendu")?;
    let model = match args.next().as_deref() {
        Some("base") => Model::Base,
        Some("large") => Model::Large,
        Some("apple") => Model::Apple,
        _ => Model::Turbo,
    };
    let language = args.next();
    let platform = oku_slides_platform::Platform::load()?;
    let (settings, storage) = (&platform.settings, &platform.storage);
    let dir = settings.paths.models();

    // La langue d'Apple s'installe d'elle-même, dans `transcribe_samples`.
    if model != Model::Apple && !model.is_installed(storage, &dir) {
        eprintln!("↓ {} …", model.label(&platform.localizer));
        let mut last = 0;
        oku_slides_stt::download(model, &dir, &settings.models, storage, |got, total| {
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
    let audio = resample(&mono, spec.sample_rate, oku_slides_stt::SAMPLE_RATE);

    let stt = Stt::new(&platform);
    // Les réglages de l'app sont remis en place à la fin.
    let before = stt.status().config;
    stt.configure(oku_slides_stt::Config { model, language, ..before.clone() });
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
