//! Moteur de dictée d'Apple (SpeechAnalyzer, macOS 26+), par le pont Swift `swift/`.
//!
//! Seul fichier `unsafe` du crate. Le pont appelle [`on_event`] depuis ses
//! propres fils, puis [`free_ctx`] une seule fois : le contexte (un
//! `Sender<Event>` en boîte) vit jusque-là.
#![expect(unsafe_code, reason = "pont FFI vers Swift : seul fichier unsafe du crate")]

use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::mpsc;

use crate::error::LocaleUnsupportedSnafu;
use crate::stream::{Availability, Event, Stream, StreamEngine};

type Callback = extern "C" fn(*mut c_void, i32, *const c_char, f64, f64);
type Free = extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn oku_speech_supported() -> bool;
    fn oku_speech_availability(locale: *const c_char) -> i32;
    fn oku_speech_install(locale: *const c_char, ctx: *mut c_void, cb: Callback, free: Free);
    fn oku_speech_start(
        locale: *const c_char,
        words: *const *const c_char,
        count: i32,
        ctx: *mut c_void,
        cb: Callback,
        free: Free,
    ) -> *mut c_void;
    fn oku_speech_push(handle: *mut c_void, samples: *const f32, count: i32);
    fn oku_speech_finish(handle: *mut c_void);
    fn oku_speech_cancel(handle: *mut c_void);
    fn oku_speech_release(handle: *mut c_void);
}

/// Un événement du pont, transmis au canal de la dictée.
extern "C" fn on_event(ctx: *mut c_void, kind: i32, text: *const c_char, a: f64, b: f64) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` vient de `Box::into_raw` d'un `Sender<Event>` (`context`) ;
    // le pont ne l'utilise plus après `free_ctx`, et sérialise ses appels.
    let tx = unsafe { &*ctx.cast::<mpsc::Sender<Event>>() };
    let text = if text.is_null() {
        String::new()
    } else {
        // SAFETY: chaîne C terminée par zéro, valide pendant l'appel (`withCString`).
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    };
    tx.send(Event::from_raw(kind, text, a, b)).ok();
}

/// Le pont a fini avec ce contexte.
extern "C" fn free_ctx(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: appelé une seule fois par le pont, après son dernier `on_event`.
    drop(unsafe { Box::from_raw(ctx.cast::<mpsc::Sender<Event>>()) });
}

/// Un canal d'événements, et son contexte pour le pont.
fn context() -> (*mut c_void, mpsc::Receiver<Event>) {
    let (tx, rx) = mpsc::channel();
    (Box::into_raw(Box::new(tx)).cast::<c_void>(), rx)
}

/// Le moteur de dictée d'Apple.
#[derive(Debug)]
pub(crate) struct AppleEngine;

impl StreamEngine for AppleEngine {
    fn supported(&self) -> bool {
        // SAFETY: fonction sans argument ni état partagé.
        unsafe { oku_speech_supported() }
    }

    fn availability(&self, locale: &str) -> Availability {
        let Ok(tag) = CString::new(locale) else { return Availability::Unsupported };
        // SAFETY: `tag` vit pendant l'appel, qui est synchrone.
        match unsafe { oku_speech_availability(tag.as_ptr()) } {
            2 => Availability::Ready,
            1 => Availability::Missing,
            _ => Availability::Unsupported,
        }
    }

    fn install(&self, locale: &str) -> mpsc::Receiver<Event> {
        let Ok(tag) = CString::new(locale) else {
            let (tx, rx) = mpsc::channel();
            tx.send(Event::Failed(LocaleUnsupportedSnafu { locale }.build().into())).ok();
            return rx;
        };
        let (ctx, rx) = context();
        // SAFETY: `tag` est copiée par le pont pendant l'appel ; il garde `ctx` jusqu'à `free_ctx`.
        unsafe { oku_speech_install(tag.as_ptr(), ctx, on_event, free_ctx) };
        rx
    }

    fn open(&self, locale: &str, vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>) {
        let tag = CString::new(locale).unwrap_or_default();
        let words: Vec<CString> = vocabulary.iter().filter_map(|w| CString::new(w.as_str()).ok()).collect();
        let pointers: Vec<*const c_char> = words.iter().map(|w| w.as_ptr()).collect();
        let count = i32::try_from(pointers.len()).unwrap_or(i32::MAX);
        let (ctx, rx) = context();
        // SAFETY: chaînes et tableau vivants pendant l'appel (copiés par le pont) ;
        // `ctx` gardé par le pont jusqu'à `free_ctx`. Handle nul : l'erreur part par `ctx`.
        let handle = unsafe { oku_speech_start(tag.as_ptr(), pointers.as_ptr(), count, ctx, on_event, free_ctx) };
        (Box::new(AppleStream { handle, finished: false, cancelled: false }), rx)
    }
}

/// Une dictée ouverte sur le pont ; relâchée au `drop`.
///
/// `cancel` reste possible après `finish` : la dictée abandonnée pendant sa
/// fin (ou trop lente à finir) n'attend plus l'analyseur.
#[derive(Debug)]
struct AppleStream {
    handle: *mut c_void,
    finished: bool,
    cancelled: bool,
}

// SAFETY: le handle désigne un objet Swift qui protège son état par un verrou
// et peut être appelé depuis n'importe quel fil.
unsafe impl Send for AppleStream {}

impl Stream for AppleStream {
    fn push(&mut self, samples: &[f32]) {
        if self.handle.is_null() || self.finished || self.cancelled {
            return;
        }
        let count = i32::try_from(samples.len()).unwrap_or(i32::MAX);
        // SAFETY: `samples` vit pendant l'appel ; le pont copie l'audio avant de rendre la main.
        unsafe { oku_speech_push(self.handle, samples.as_ptr(), count) };
    }

    fn finish(&mut self) {
        if self.handle.is_null() || self.finished || self.cancelled {
            return;
        }
        self.finished = true;
        // SAFETY: handle valide jusqu'à `oku_speech_release` (dans `drop`).
        unsafe { oku_speech_finish(self.handle) };
    }

    fn cancel(&mut self) {
        if self.handle.is_null() || self.cancelled {
            return;
        }
        self.cancelled = true;
        // SAFETY: handle valide jusqu'à `oku_speech_release` (dans `drop`) ;
        // le pont accepte l'annulation après `finish`, et même après `Done`.
        unsafe { oku_speech_cancel(self.handle) };
    }
}

impl Drop for AppleStream {
    fn drop(&mut self) {
        // Ni fini ni annulé : on annule, sinon l'analyseur attendrait de l'audio pour toujours.
        if !self.finished && !self.cancelled {
            self.cancel();
        }
        if !self.handle.is_null() {
            // SAFETY: relâche la référence prise par `oku_speech_start`, une seule fois.
            unsafe { oku_speech_release(self.handle) };
        }
    }
}

#[cfg(test)]
#[path = "apple_tests.rs"]
mod apple_tests;
