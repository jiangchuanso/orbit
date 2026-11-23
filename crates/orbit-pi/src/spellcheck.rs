//! Spelling underlines in the prompt composer.
//!
//! macOS spell-checks a text view for free only because an `NSTextView` asks
//! it to. Orbit's composer is not one: it shapes and paints its own text
//! (`composer.rs`), so the platform never sees the words. This module is the
//! missing half — it decides which spans of a prompt are prose worth checking
//! ([`spellable_spans`]) and asks `NSSpellChecker` for the misspelled words in
//! them ([`misspelled_ranges`]), so the composer can paint a wavy underline
//! under each one.
//!
//! Two things are deliberately **not** wired:
//!
//! - **Grammar.** `NSSpellChecker.checkGrammarOfString` exists, but a pi prompt
//!   is a stream of imperatives and code fragments, which is exactly the style
//!   Apple's grammar checker flags. It would be red most of the time and
//!   therefore ignored, so shipping it would make a real signal worthless.
//! - **Auto-correction.** Rewriting a prompt behind the user's back is worse
//!   than a missed typo, and a prompt is not prose: a "corrected" identifier
//!   breaks the task.
//!
//! The checker is a synchronous AppKit call, so it runs from the composer's
//! debounced task (a pause in typing), never from a paint pass, and only once
//! [`start`] has marked the process as the real windowed app — a unit test has
//! no window server and must not touch AppKit off the main thread.

use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set once the real app is running on the platform's main thread.
///
/// `NSSpellChecker` is main-thread AppKit. `cargo test` runs each test on a
/// spawned thread, so the checker is never reached from there; the platform
/// call is gated on this flag instead of on a test-only branch.
static PLATFORM_READY: AtomicBool = AtomicBool::new(false);

/// Mark the process as the running app, so the platform checker may be used.
/// Called from the window bootstrap, on the main thread.
pub(crate) fn start() {
    PLATFORM_READY.store(true, Ordering::Relaxed);
}

/// Whether this build can spell-check and the user asked for it.
///
/// The preference lives in `ui.json` (`UiPrefs::spell_check`) and is read fresh
/// on every render, so flipping the switch in Settings reaches an open composer
/// without rebuilding it.
pub(crate) fn enabled(cx: &gpui::App) -> bool {
    cfg!(target_os = "macos") && crate::theme::get(cx).ui.spell_check
}

/// Whether the platform path is live (macOS, and the app is actually running).
fn platform_ready() -> bool {
    cfg!(target_os = "macos") && PLATFORM_READY.load(Ordering::Relaxed)
}

/// Misspelled byte ranges in `text`, sorted and non-overlapping.
///
/// Empty when spelling is unavailable, so a caller can paint the result
/// unconditionally. Every range is a valid UTF-8 slice boundary of `text`.
pub(crate) fn misspelled_ranges(text: &str) -> Vec<Range<usize>> {
    if text.is_empty() || !platform_ready() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for span in spellable_spans(text) {
        for found in check_span(&text[span.clone()]) {
            out.push(span.start + found.start..span.start + found.end);
        }
    }
    out.sort_by_key(|range| range.start);
    out
}

/// Keep only ranges that still fit `text` on character boundaries.
///
/// Ranges are computed for the content as it was at the last check, and the
/// buffer may have changed since — typing keeps the previous marks on screen
/// instead of blinking every squiggle off on each keystroke. A stale range can
/// therefore point past the end or into the middle of a character, and a run
/// boundary there would split a glyph. Clamping is cheaper than re-checking.
pub(crate) fn clamp_ranges(text: &str, ranges: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = ranges
        .iter()
        .filter(|range| {
            range.start < range.end
                && range.end <= text.len()
                && text.is_char_boundary(range.start)
                && text.is_char_boundary(range.end)
        })
        .cloned()
        .collect();
    out.sort_by_key(|range| range.start);
    out
}

/// Replacement candidates for `word`, best first (at most [`MAX_GUESSES`]).
///
/// Empty when spelling is unavailable or the platform has no suggestion — the
/// caller shows that as "no guesses" rather than inventing one.
pub(crate) fn guesses(word: &str) -> Vec<String> {
    if word.is_empty() || !platform_ready() {
        return Vec::new();
    }
    let mut out = platform_guesses(word);
    out.truncate(MAX_GUESSES);
    out
}

/// Add `word` to the platform's spelling dictionary, so it stops being marked.
///
/// This is the user's own dictionary (macOS keeps it in `~/Library/Spelling`),
/// shared with the apps that use it — exactly what "Learn spelling" means in a
/// native text view. It only ever runs from that menu row.
pub(crate) fn learn(word: &str) {
    if word.is_empty() || !platform_ready() {
        return;
    }
    platform_learn(word);
}

/// How many suggestions a right-click menu offers; a longer list would push the
/// word being replaced out of view.
const MAX_GUESSES: usize = 5;

// ── which spans are prose ───────────────────────────────────────────────

/// Punctuation that may wrap a word without making it code: `(bug)`, `"bug"`,
/// `**bug**`, `bug.`, `bug,`.
const WRAPPING_PUNCTUATION: &[char] = &[
    '.', ',', ';', ':', '!', '?', '"', '\'', '’', '‘', '“', '”', '(', ')', '[', ']', '{', '}', '*',
    '_', '-', '#', '«', '»', '„',
];

/// Ranges of `text` handed to the platform checker: contiguous runs of prose
/// words, one per line, broken by anything that reads as code.
///
/// A run keeps its internal spaces so the checker sees word context
/// ("teh bug" rather than two isolated words), and a run never crosses a
/// skipped token — a prompt like `fix teh bug in src/main.rs` yields two runs,
/// so the path contributes no misspellings.
pub(crate) fn spellable_spans(text: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    // ``` / ~~~ fences carry code, and their contents are the loudest source
    // of false positives in a prompt.
    let mut in_fence = false;
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let starts_fence = {
            let trimmed = body.trim_start();
            trimmed.starts_with("```") || trimmed.starts_with("~~~")
        };
        if starts_fence {
            in_fence = !in_fence;
        } else if !in_fence {
            collect_line_spans(body, line_start, &mut spans);
        }
        line_start = line_end;
    }
    spans
}

/// One line's prose runs, offset by `base`.
fn collect_line_spans(body: &str, base: usize, out: &mut Vec<Range<usize>>) {
    let mut in_code = false;
    let mut run: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < body.len() {
        let ch = body[i..].chars().next().unwrap_or('\0');
        if ch == '`' {
            // An inline code span: `cmd --flag`. A run on either side of it is
            // its own span, so the checked text never contains the code.
            close_run(&mut run, base, out);
            in_code = !in_code;
            while body[i..].starts_with('`') {
                i += 1;
            }
            continue;
        }
        if ch.is_whitespace() {
            i += ch.len_utf8();
            continue;
        }
        let token_start = i;
        while i < body.len() {
            let c = body[i..].chars().next().unwrap_or('\0');
            if c.is_whitespace() || c == '`' {
                break;
            }
            i += c.len_utf8();
        }
        let token = &body[token_start..i];
        if in_code || !is_prose_token(token) {
            close_run(&mut run, base, out);
            continue;
        }
        match &mut run {
            Some((_, end)) => *end = i,
            None => run = Some((token_start, i)),
        }
    }
    close_run(&mut run, base, out);
}

fn close_run(run: &mut Option<(usize, usize)>, base: usize, out: &mut Vec<Range<usize>>) {
    if let Some((start, end)) = run.take() {
        out.push(base + start..base + end);
    }
}

/// Whether one whitespace-delimited token is an English word worth checking
/// rather than code, a path, an identifier or a command.
///
/// The rules are structural rather than a dictionary: a prompt's code tokens
/// are recognisable by shape. False negatives (a real typo inside something
/// that looks like code) are the right failure direction here — a missing
/// squiggle is quiet, a wrong one teaches the user to ignore all of them.
fn is_prose_token(token: &str) -> bool {
    let core = token.trim_matches(WRAPPING_PUNCTUATION);
    if core.chars().count() < 2 {
        return false;
    }
    // Anything other than a letter, a digit, an apostrophe or a hyphen is
    // code punctuation: `/ \ @ $ = < > _ . : ; ( ) [ ] { } | & ~ ^ % + * #`.
    if core
        .chars()
        .any(|c| !c.is_alphanumeric() && !matches!(c, '\'' | '’' | '-'))
    {
        return false;
    }
    if !core.chars().any(char::is_alphabetic) {
        return false;
    }
    // `v2`, `mp3`, an issue number, a version.
    if core.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    // `camelCase`, `PascalCase` — identifiers, not words.
    let chars: Vec<char> = core.chars().collect();
    if chars
        .windows(2)
        .any(|pair| pair[0].is_lowercase() && pair[1].is_uppercase())
    {
        return false;
    }
    // `API`, `JSON`, `TODO` — acronyms.
    let alphabetic = chars.iter().filter(|c| c.is_alphabetic()).count();
    let all_caps = alphabetic >= 2
        && chars
            .iter()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase());
    !all_caps
}

// ── the platform checker ────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn check_span(span: &str) -> Vec<Range<usize>> {
    use objc2_app_kit::NSSpellChecker;
    use objc2_foundation::NSString;

    let checker = NSSpellChecker::sharedSpellChecker();
    let string = NSString::from_str(span);
    let length = string.length();
    let mut out = Vec::new();
    let mut from = 0usize;
    // `checkSpellingOfString` answers the *first* misspelling at or after
    // `from` (no wrap, so it stops at the end of the string), which is why this
    // loops. Each word is one round trip; a prompt is a few dozen at most.
    while from < length {
        let mut word_count: isize = 0;
        // SAFETY: `word_count` is a live, aligned out-parameter and `string`
        // outlives the call; the selector reads the string and writes only
        // through the pointer passed here.
        let found = unsafe {
            checker.checkSpellingOfString_startingAt_language_wrap_inSpellDocumentWithTag_wordCount(
                &string,
                from as isize,
                None,
                false,
                0,
                &mut word_count,
            )
        };
        // `NSNotFound` is `NSUInteger::MAX` and is not re-exported by
        // objc2-foundation, so it is compared as the constant it is.
        if found.location == usize::MAX || found.length == 0 {
            break;
        }
        let next = found.location.saturating_add(found.length);
        if next <= from {
            break;
        }
        if let Some(range) = utf16_range_to_bytes(span, found.location..next) {
            out.push(range);
        }
        from = next;
    }
    out
}

#[cfg(not(target_os = "macos"))]
fn check_span(_span: &str) -> Vec<Range<usize>> {
    // Windows (`ISpellChecker`) and Linux have no checker in this stack; the
    // Settings row is macOS-only rather than a switch that does nothing.
    Vec::new()
}

#[cfg(target_os = "macos")]
fn platform_guesses(word: &str) -> Vec<String> {
    use objc2_app_kit::NSSpellChecker;
    use objc2_foundation::{NSRange, NSString};

    let checker = NSSpellChecker::sharedSpellChecker();
    let string = NSString::from_str(word);
    let range = NSRange::new(0, string.length());
    checker
        .guessesForWordRange_inString_language_inSpellDocumentWithTag(range, &string, None, 0)
        .map(|guesses| {
            guesses
                .to_vec()
                .iter()
                .map(|guess| guess.to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
fn platform_guesses(_word: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn platform_learn(word: &str) {
    use objc2_app_kit::NSSpellChecker;
    use objc2_foundation::NSString;

    NSSpellChecker::sharedSpellChecker().learnWord(&NSString::from_str(word));
}

#[cfg(not(target_os = "macos"))]
fn platform_learn(_word: &str) {}

/// The byte range in `text` for a UTF-16 range into it, or `None` when the
/// range does not land on character boundaries.
///
/// `NSSpellChecker` counts UTF-16 code units (what `NSString` sees); the
/// composer counts bytes.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn utf16_range_to_bytes(text: &str, want: Range<usize>) -> Option<Range<usize>> {
    if want.start >= want.end {
        return None;
    }
    let mut start = None;
    let mut end = None;
    let mut utf16 = 0usize;
    for (index, ch) in text.char_indices() {
        if utf16 == want.start && start.is_none() {
            start = Some(index);
        }
        if utf16 == want.end && end.is_none() {
            end = Some(index);
        }
        utf16 += ch.len_utf16();
    }
    if start.is_none() && utf16 == want.start {
        start = Some(text.len());
    }
    if end.is_none() && utf16 == want.end {
        end = Some(text.len());
    }
    match (start, end) {
        (Some(start), Some(end)) if start < end => Some(start..end),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<&str> {
        spellable_spans(text)
            .into_iter()
            .map(|range| &text[range])
            .collect()
    }

    #[test]
    fn prose_is_one_span_so_the_checker_sees_word_context() {
        assert_eq!(
            spans("Fix teh bug in the parser."),
            vec!["Fix teh bug in the parser."]
        );
    }

    /// A path, an identifier, a mention and a slash command each break the run
    /// instead of joining it — the checker never reads them as English.
    #[test]
    fn code_shaped_tokens_break_the_run() {
        assert_eq!(spans("fix teh bug in src/main.rs"), vec!["fix teh bug in"]);
        assert_eq!(spans("check getEntries now"), vec!["check", "now"]);
        assert_eq!(spans("ask @README about it"), vec!["ask", "about it"]);
        assert_eq!(spans("run /model first"), vec!["run", "first"]);
        assert_eq!(spans("set FOO_BAR then retry"), vec!["set", "then retry"]);
    }

    #[test]
    fn fenced_and_inline_code_is_never_checked() {
        assert!(spans("```rust\nlet teh = 1;\n```").is_empty());
        assert_eq!(
            spans("call `git status` and wait"),
            vec!["call", "and wait"]
        );
        // A fence closes again, so the prose after it is checked.
        assert_eq!(spans("```\ncode\n```\nteh end"), vec!["teh end"]);
    }

    #[test]
    fn acronyms_versions_and_identifiers_are_not_words() {
        for token in [
            "API",
            "JSON",
            "v2",
            "mp3",
            "camelCase",
            "getEntries",
            "a",
            "42",
        ] {
            assert!(!is_prose_token(token), "{token} should not be checked");
        }
        for token in ["bug", "parser", "don't", "well-known", "(teh)", "**Fix**"] {
            assert!(is_prose_token(token), "{token} should be checked");
        }
    }

    /// A stale mark may point past an edit or into the middle of a character;
    /// only ranges that are still valid boundaries survive.
    #[test]
    fn clamp_drops_ranges_a_later_edit_invalidated() {
        let text = "héllo";
        let one = std::slice::from_ref;
        assert_eq!(clamp_ranges(text, one(&(0..5))), vec![0..5]);
        // `é` is two bytes: 2..3 is inside it.
        assert!(clamp_ranges(text, one(&(2..3))).is_empty());
        // Past the end.
        assert!(clamp_ranges(text, one(&(0..99))).is_empty());
        assert!(clamp_ranges(text, one(&(3..3))).is_empty());
    }

    #[test]
    fn utf16_ranges_map_onto_byte_offsets() {
        let text = "a\u{1F600}b"; // 1 + 2 bytes + 1; UTF-16 is 1 + 2 + 1 units.
        assert_eq!(utf16_range_to_bytes(text, 0..1), Some(0..1));
        assert_eq!(utf16_range_to_bytes(text, 3..4), Some(5..6));
        // A range that splits the surrogate pair is not a byte range.
        assert_eq!(utf16_range_to_bytes(text, 1..2), None);
        // Ending exactly at the string's end.
        assert_eq!(utf16_range_to_bytes(text, 1..4), Some(1..6));
    }

    /// Nothing reaches AppKit until the real app says so, so a unit test cannot
    /// call a main-thread-only API from a test thread.
    #[test]
    fn the_platform_checker_is_dark_before_start() {
        let none: Vec<Range<usize>> = Vec::new();
        assert_eq!(misspelled_ranges("teh bug"), none);
        // The context menu's two platform calls are gated the same way: no
        // suggestions, and learning a word is a no-op rather than a write into
        // the user's dictionary from a test process.
        assert!(guesses("teh").is_empty());
        learn("orbit");
    }
}
