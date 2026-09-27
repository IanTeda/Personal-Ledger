//! What the terminal can draw and whether its background is light or dark (ADR-0024).
//!
//! Colour depth comes from `COLORTERM`/`TERM`. The System Colour Variant comes from asking the
//! terminal once at start, before crossterm's input reader exists: DEC mode 2031's colour-scheme
//! report (`CSI ? 996 n`), then the background colour (OSC 11), then `COLORFGBG`. Undetected
//! means `None`, which `resolve` draws as Dark.
//!
//! Live mode 2031 notifications are not followed: crossterm 0.29's parser treats an unknown
//! `CSI ?` sequence as incomplete and would swallow every key after it.

use std::time::Duration;

use lib_colour_theme::ColourVariant;

use crate::colours::ColourDepth;

/// How long the probe waits for the terminal to answer. Long enough for a local or SSH round
/// trip, short enough that a terminal which never answers doesn't delay start noticeably.
const PROBE_TIMEOUT: Duration = Duration::from_millis(150);

/// The colour depth the environment advertises. `COLORTERM` is the only reliable truecolor
/// signal; `TERM` otherwise says 256 or not.
pub fn colour_depth(colorterm: Option<&str>, term: Option<&str>) -> ColourDepth {
    if matches!(colorterm, Some("truecolor" | "24bit")) {
        return ColourDepth::TrueColor;
    }
    match term {
        Some(term) if term.contains("256color") || term.contains("direct") => ColourDepth::Ansi256,
        // Windows consoles set no `TERM` and have drawn RGB since Windows 10.
        None if cfg!(windows) => ColourDepth::TrueColor,
        _ => ColourDepth::Ansi16,
    }
}

/// Reads the colour depth from the process environment.
pub fn colour_depth_from_env() -> ColourDepth {
    let colorterm = std::env::var("COLORTERM").ok();
    let term = std::env::var("TERM").ok();
    colour_depth(colorterm.as_deref(), term.as_deref())
}

/// The System Colour Variant from a probe reply and `COLORFGBG`, in ADR-0024's order.
pub fn system_variant(reply: &[u8], colorfgbg: Option<&str>) -> Option<ColourVariant> {
    parse_colour_scheme_report(reply)
        .or_else(|| parse_osc11(reply))
        .or_else(|| colorfgbg.and_then(parse_colorfgbg))
}

/// Asks the terminal, then falls back to `COLORFGBG`. Only call before crossterm's event
/// reader starts, or the reply is read as key presses.
pub fn system_variant_from_terminal() -> Option<ColourVariant> {
    let reply = probe().unwrap_or_default();
    let colorfgbg = std::env::var("COLORFGBG").ok();
    let variant = system_variant(&reply, colorfgbg.as_deref());
    tracing::debug!(
        ?variant,
        reply_len = reply.len(),
        "System Colour Variant detected"
    );
    variant
}

/// DEC mode 2031's reply `CSI ? 997 ; 1 n` (dark) or `; 2 n` (light).
fn parse_colour_scheme_report(reply: &[u8]) -> Option<ColourVariant> {
    if find(reply, b"\x1b[?997;1n").is_some() {
        Some(ColourVariant::Dark)
    } else if find(reply, b"\x1b[?997;2n").is_some() {
        Some(ColourVariant::Light)
    } else {
        None
    }
}

/// OSC 11's reply `OSC 11 ; rgb:RRRR/GGGG/BBBB ST`, each channel 1–4 hex digits, ended by
/// BEL or `ESC \`. Light when the background's luma is over half.
fn parse_osc11(reply: &[u8]) -> Option<ColourVariant> {
    const PREFIX: &[u8] = b"\x1b]11;rgb:";
    let start = find(reply, PREFIX)? + PREFIX.len();
    let rest = &reply[start..];
    let end = rest.iter().position(|b| *b == 0x07 || *b == 0x1b)?;
    let body = std::str::from_utf8(&rest[..end]).ok()?;

    let mut channels = body.split('/').map(|hex| {
        if hex.is_empty() || hex.len() > 4 {
            return None;
        }
        let value = u32::from_str_radix(hex, 16).ok()?;
        let max = (1u32 << (4 * hex.len())) - 1;
        Some(f64::from(value) / f64::from(max))
    });
    let (r, g, b) = (channels.next()??, channels.next()??, channels.next()??);
    if channels.next().is_some() {
        return None;
    }
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    Some(if luma > 0.5 {
        ColourVariant::Light
    } else {
        ColourVariant::Dark
    })
}

/// `COLORFGBG` is `fg;bg` (rxvt adds a middle field): the last field is the background's ANSI
/// index. 7 (white) and the bright colours bar 8 (bright black) are light backgrounds.
fn parse_colorfgbg(value: &str) -> Option<ColourVariant> {
    let background: u8 = value.rsplit(';').next()?.trim().parse().ok()?;
    match background {
        7 | 9..=15 => Some(ColourVariant::Light),
        0..=6 | 8 => Some(ColourVariant::Dark),
        _ => None,
    }
}

/// Whether the reply holds the primary device attributes answer `CSI ? … c`. The probe asks
/// for it last, and terminals answer in order, so it marks the end of everything they will
/// say — a terminal without OSC 11 or mode 2031 still answers it, so the probe needn't wait
/// for the timeout.
fn has_device_attributes(reply: &[u8]) -> bool {
    let mut rest = reply;
    while let Some(start) = find(rest, b"\x1b[?") {
        rest = &rest[start + 3..];
        let params = rest
            .iter()
            .take_while(|b| b.is_ascii_digit() || **b == b';')
            .count();
        if rest.get(params) == Some(&b'c') {
            return true;
        }
    }
    false
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Writes the queries and collects the reply until device attributes arrive or the timeout
/// passes. Raw mode is held only for the probe, before the alternate screen, and raw mode
/// turns echo off, so nothing the terminal sends back reaches the screen.
#[cfg(unix)]
fn probe() -> Option<Vec<u8>> {
    use std::io::{IsTerminal as _, Write as _};
    use std::time::Instant;

    use rustix::event::{PollFd, PollFlags, Timespec, poll};

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    if !stdin.is_terminal() || !stdout.is_terminal() {
        return None;
    }

    crossterm::terminal::enable_raw_mode().ok()?;
    let mut reply = Vec::new();
    let written = stdout
        .write_all(b"\x1b[?996n\x1b]11;?\x1b\\\x1b[c")
        .and_then(|()| stdout.flush());
    if written.is_ok() {
        let deadline = Instant::now() + PROBE_TIMEOUT;
        let mut buf = [0u8; 256];
        while !has_device_attributes(&reply) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            let timeout = Timespec::try_from(left).ok();
            let mut fds = [PollFd::new(&stdin, PollFlags::IN)];
            match poll(&mut fds, timeout.as_ref()) {
                Ok(0) | Err(_) => break,
                Ok(_) => match rustix::io::read(&stdin, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => reply.extend_from_slice(&buf[..read]),
                },
            }
        }
    }
    let _ = crossterm::terminal::disable_raw_mode();
    Some(reply)
}

#[cfg(not(unix))]
fn probe() -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorterm_truecolor_wins() {
        assert_eq!(
            colour_depth(Some("truecolor"), Some("xterm")),
            ColourDepth::TrueColor
        );
        assert_eq!(colour_depth(Some("24bit"), None), ColourDepth::TrueColor);
    }

    #[test]
    fn term_256color_is_256() {
        assert_eq!(
            colour_depth(None, Some("xterm-256color")),
            ColourDepth::Ansi256
        );
        assert_eq!(
            colour_depth(Some(""), Some("tmux-256color")),
            ColourDepth::Ansi256
        );
    }

    #[test]
    fn plain_term_is_16() {
        assert_eq!(colour_depth(None, Some("linux")), ColourDepth::Ansi16);
        assert_eq!(colour_depth(None, Some("xterm")), ColourDepth::Ansi16);
    }

    #[test]
    fn osc11_reads_4_2_and_1_digit_channels_with_either_terminator() {
        assert_eq!(
            parse_osc11(b"\x1b]11;rgb:1616/1414/1313\x1b\\"),
            Some(ColourVariant::Dark)
        );
        assert_eq!(
            parse_osc11(b"\x1b]11;rgb:fa/f8/f5\x07"),
            Some(ColourVariant::Light)
        );
        assert_eq!(
            parse_osc11(b"\x1b]11;rgb:f/f/f\x07"),
            Some(ColourVariant::Light)
        );
    }

    #[test]
    fn osc11_rejects_malformed_replies() {
        assert_eq!(parse_osc11(b"\x1b]11;rgb:ffff/ffff\x07"), None);
        assert_eq!(parse_osc11(b"\x1b]11;rgb:fffff/0/0\x07"), None);
        assert_eq!(parse_osc11(b"\x1b]11;rgb:zz/00/00\x07"), None);
        // Cut off before its terminator.
        assert_eq!(parse_osc11(b"\x1b]11;rgb:ffff/ffff/ffff"), None);
        assert_eq!(parse_osc11(b""), None);
    }

    #[test]
    fn colorfgbg_reads_the_last_field_as_the_background() {
        assert_eq!(parse_colorfgbg("15;0"), Some(ColourVariant::Dark));
        assert_eq!(parse_colorfgbg("0;15"), Some(ColourVariant::Light));
        assert_eq!(parse_colorfgbg("0;default;7"), Some(ColourVariant::Light));
        assert_eq!(parse_colorfgbg("7;8"), Some(ColourVariant::Dark));
        assert_eq!(parse_colorfgbg("default;default"), None);
        assert_eq!(parse_colorfgbg(""), None);
    }

    #[test]
    fn colour_scheme_report_beats_osc11_which_beats_colorfgbg() {
        let osc_light = b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\\x1b[?62;22c";
        let both = b"\x1b[?997;1n\x1b]11;rgb:ffff/ffff/ffff\x1b\\\x1b[?62;22c";
        assert_eq!(
            system_variant(both, Some("0;15")),
            Some(ColourVariant::Dark)
        );
        assert_eq!(
            system_variant(osc_light, Some("15;0")),
            Some(ColourVariant::Light)
        );
        assert_eq!(
            system_variant(b"\x1b[?62;22c", Some("0;15")),
            Some(ColourVariant::Light)
        );
        assert_eq!(system_variant(b"", None), None);
    }

    #[test]
    fn device_attributes_end_the_reply_but_the_2031_report_does_not() {
        assert!(has_device_attributes(b"\x1b[?997;2n\x1b[?1;2c"));
        assert!(has_device_attributes(b"\x1b[?c"));
        assert!(!has_device_attributes(b"\x1b[?997;2n"));
        assert!(!has_device_attributes(b"\x1b]11;rgb:0/0/0\x07"));
    }
}
