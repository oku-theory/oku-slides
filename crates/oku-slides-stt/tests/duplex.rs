//! The macOS duplex audio for real, by hand: `cargo test -p oku-slides-stt -- --ignored duplex`.
//!
//! Two scenarios: the duplex speaks and captures; then hands-free listening through the duplex
//! (barge-in on) never takes Oku's own voice for the user while it speaks (echo cancellation).
//! Keep the room quiet: a voice in the room would count as the user.
//!
//! Its own `main` (`harness = false`): `AVSpeechSynthesizer.write` delivers its buffers on the main
//! queue, which only the main thread's run loop drains, and libtest keeps the main thread. The
//! scenario runs on a worker while the main thread runs the loop. Answers libtest's and nextest's
//! `--list` and `--ignored` like an ignored test, so default runs skip it. Needs speakers, a
//! microphone and the microphone permission for the host (the terminal).

#[cfg(target_os = "macos")]
mod scenario {
    use std::sync::mpsc;
    use std::time::Duration;

    use oku_slides_stt::{Duplex, SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus};

    /// Speaks a short text through the duplex: DONE within 10 s, and the microphone heard something.
    pub(crate) fn duplex_speaks_and_captures() -> Result<(), String> {
        let duplex = Duplex::start().map_err(|e| format!("duplex start: {e}"))?;
        let voice = duplex.voice();
        let SpeechStatus::Ready { voices } = voice.status("en") else { return Err("no English voice".into()) };
        let id = voices.first().map(|v| v.id.clone()).ok_or("no English voice")?;
        let (tx, rx) = mpsc::channel();
        // The receiver may be gone after a timeout: a late DONE then has nobody to tell.
        let done = SpeechDone::new(move |id| tx.send(id).unwrap_or_default());
        voice
            .speak(SpeechId::new(1), "Hello from Oku.", Some(&id), SpeechRate::default(), done)
            .map_err(|e| format!("speak: {e}"))?;
        let ended = rx.recv_timeout(Duration::from_secs(10));
        let level = duplex.level();
        eprintln!("DONE: {ended:?}, microphone level {level}");
        if ended != Ok(SpeechId::new(1)) {
            return Err(format!("no DONE within 10 s: {ended:?}"));
        }
        if level <= 0.0 {
            return Err("the microphone level stayed at 0".into());
        }
        // One DONE per id: nothing more once the speech is over and cut.
        voice.stop(SpeechId::new(1)).map_err(|e| format!("stop: {e}"))?;
        if let Ok(again) = rx.try_recv() {
            return Err(format!("a second DONE: {again:?}"));
        }
        Ok(())
    }

    /// Listens through the duplex while Oku speaks a long text: the user never seems to speak.
    pub(crate) fn own_voice_never_heard_as_the_user() -> Result<(), String> {
        use std::time::Instant;

        use oku_slides_i18n::Localizer;
        use oku_slides_platform::Platform;
        use oku_slides_platform::config::Settings;
        use oku_slides_stt::{
            ApiConfig, Config, Model, Speaker, Speech, SpeechEnded, State, Stt, system_speech_engine,
        };

        let dir = tempfile::tempdir().map_err(|e| format!("temp dir: {e}"))?;
        let mut settings = Settings::defaults().map_err(|e| format!("settings: {e}"))?;
        settings.paths.data = Some(dir.path().join("data"));
        settings.paths.decks = Some(dir.path().join("decks"));
        settings.paths.models = Some(dir.path().join("models"));
        settings.i18n.locale = Some("en".into());
        settings.stt.apple = false;
        let platform = Platform::with_settings(settings).map_err(|e| format!("platform: {e}"))?;
        let stt = Stt::new(&platform);
        // The API engine on a closed loopback port: built at once, called only for a phrase,
        // which must not come. `PATH` is always set, so the key is present.
        let api = ApiConfig { url: "http://127.0.0.1:9/v1".into(), key_env: "PATH".into(), ..ApiConfig::default() };
        let config = Config { model: Model::Api, language: Some("en".into()), api, ..Config::default() };
        if !stt.configure(config) {
            return Err("dictation settings refused".into());
        }
        stt.listen(Vec::new());
        let since = Instant::now();
        while stt.status().state != State::Listening {
            if since.elapsed() > Duration::from_secs(10) {
                return Err(format!("not listening: {:?}", stt.status().error));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if let Some(reason) = stt.barge_in_unavailable(&Localizer::new(Some("en"))) {
            return Err(format!("barge-in unavailable: {reason}"));
        }
        // Let the voice processing settle on the room before Oku speaks.
        std::thread::sleep(Duration::from_secs(1));
        let speaker = Speaker::new(system_speech_engine(), Some(stt.clone()));
        let (tx, rx) = mpsc::channel();
        let ended = SpeechEnded::new(move |at| tx.send(at).unwrap_or_default());
        let text = "Here is your deck: three slides were added, the title now reads quarterly results, \
                    and the chart on the second slide shows revenue by region for the last four quarters.";
        let started = Instant::now();
        speaker.say(Speech {
            text: text.into(),
            lang: "en".into(),
            voice: None,
            rate: oku_slides_stt::SpeechRate::default(),
            ended: Some(ended),
            asks: false,
        });
        let (mut polls, mut open_polls, mut level) = (0_u32, 0_u32, 0.0_f32);
        let end = loop {
            if let Ok(at) = rx.try_recv() {
                break at;
            }
            if started.elapsed() > Duration::from_secs(30) {
                return Err("the speech did not end within 30 s".into());
            }
            if stt.user_speaking() {
                return Err(format!("Oku's own voice was taken for the user after {:?}", started.elapsed()));
            }
            polls += 1;
            let status = stt.status();
            if !status.suspended {
                open_polls += 1;
            }
            level = level.max(status.level);
            std::thread::sleep(Duration::from_millis(10));
        };
        let played = end.saturating_duration_since(started);
        eprintln!("speech played {played:?}; {polls} polls, microphone open in {open_polls}, top level {level}");
        let status = stt.status();
        stt.stop();
        if status.state != State::Listening {
            return Err(format!("listening ended during the speech: {:?}", status.error));
        }
        if played < Duration::from_secs(3) {
            return Err(format!("the speech ended after {played:?}: too short to have played"));
        }
        if level <= 0.0 {
            return Err("listening heard no frame: the check proves nothing".into());
        }
        if open_polls != polls {
            return Err(format!("listening was suspended in {} polls: not barge-in", polls - open_polls));
        }
        Ok(())
    }
}

/// The one test of this binary.
const NAME: &str = "duplex_speaks_and_captures";

/// libtest options followed by a value, which is no name filter.
const VALUED: [&str; 6] = ["--format", "--test-threads", "--skip", "--logfile", "--color", "-Z"];

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let mut filters = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if VALUED.contains(&arg.as_str()) {
            rest.next();
        } else if !arg.starts_with('-') {
            filters.push(arg.as_str());
        }
    }
    let selected =
        filters.is_empty() || filters.iter().any(|f| if has("--exact") { *f == NAME } else { NAME.contains(f) });
    if has("--list") {
        if cfg!(target_os = "macos") && selected {
            println!("{NAME}: test");
        }
        return std::process::ExitCode::SUCCESS;
    }
    if !cfg!(target_os = "macos") || !selected {
        return std::process::ExitCode::SUCCESS;
    }
    if !has("--ignored") && !has("--include-ignored") {
        println!("test {NAME} ... ignored, plays speech and opens the microphone: run by hand on macOS");
        return std::process::ExitCode::SUCCESS;
    }
    run()
}

#[cfg(target_os = "macos")]
fn run() -> std::process::ExitCode {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use objc2_core_foundation::CFRunLoop;

    let finished = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&finished);
    let worker = std::thread::spawn(move || {
        let result =
            scenario::duplex_speaks_and_captures().and_then(|()| scenario::own_voice_never_heard_as_the_user());
        flag.store(true, Ordering::SeqCst);
        // Until the main thread leaves its loop: a stop sent before the loop runs is lost.
        loop {
            if let Some(main) = CFRunLoop::main() {
                main.stop();
            }
            if result.is_err() || Arc::strong_count(&flag) == 1 {
                return result;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
    while !finished.load(Ordering::SeqCst) {
        CFRunLoop::run();
    }
    drop(finished);
    match worker.join() {
        Ok(Ok(())) => {
            println!("test {NAME} ... ok");
            std::process::ExitCode::SUCCESS
        }
        Ok(Err(e)) => {
            eprintln!("test {NAME} ... FAILED: {e}");
            std::process::ExitCode::FAILURE
        }
        Err(_) => {
            eprintln!("test {NAME} ... FAILED: panicked");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn run() -> std::process::ExitCode {
    std::process::ExitCode::SUCCESS
}
