//! Resume decisions of a model download: answers to a range request, retryable failures.

/// Bytes the hub must add to a `.part` before the retry counter starts over.
pub(super) const PROGRESS_STEP: u64 = 1 << 20;

/// What to do with the body of an answer, given the offset asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Answer {
    /// The body continues the `.part`; `total` is the whole file's size, when known.
    Append { total: Option<u64> },
    /// The body is the whole file: the `.part` is rewritten from zero.
    Fresh { total: Option<u64> },
    /// A partial body that does not continue the `.part`: drop it and ask again from zero.
    Refuse,
}

/// Classifies an answer to a request asked `from` an offset (0: no `Range` sent).
///
/// `length` is the `Content-Length`, `content_range` the raw `Content-Range` header.
pub(super) fn answer(from: u64, status: u16, length: Option<u64>, content_range: Option<&str>) -> Answer {
    if status != 206 {
        return Answer::Fresh { total: length };
    }
    if from == 0 {
        return Answer::Refuse;
    }
    let Some((end, total)) = content_range.and_then(|header| parse_content_range(header, from)) else {
        return Answer::Refuse;
    };
    let Some(next) = end.checked_add(1) else {
        return Answer::Refuse;
    };
    // A length that disagrees with the range is a broken answer.
    if length.is_some_and(|l| Some(l) != next.checked_sub(from)) {
        return Answer::Refuse;
    }
    Answer::Append { total: Some(total.unwrap_or(next)) }
}

/// `bytes L-E/T` starting at `start`: `(E, Some(T))`, or `(E, None)` for `T` = `*`.
///
/// Refuses another start, `E < L`, and a total not above `E`. A total above `E + 1` is a
/// server capping the slice it sends (RFC 9110): the rest comes with the next request.
pub(super) fn parse_content_range(header: &str, start: u64) -> Option<(u64, Option<u64>)> {
    let (range, total) = header.trim().strip_prefix("bytes ")?.split_once('/')?;
    let (first, last) = range.split_once('-')?;
    let (first, last): (u64, u64) = (first.parse().ok()?, last.parse().ok()?);
    if first != start || last < first {
        return None;
    }
    match total {
        "*" => Some((last, None)),
        total => {
            let total: u64 = total.parse().ok()?;
            (last < total).then_some((last, Some(total)))
        }
    }
}

/// A failed request worth trying again: busy hub, dropped connection or timeout.
pub(super) fn retryable(error: &ureq::Error) -> bool {
    matches!(error, ureq::Error::StatusCode(429 | 500 | 502 | 503 | 504) | ureq::Error::Io(_) | ureq::Error::Timeout(_))
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod resume_tests;
