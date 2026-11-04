//! The Transactions table's status marks and the status line's legend for them, in the Status
//! glyphs Display Preference. The page's Presenter puts each row's marks and the legend in its view
//! model, so a Client draws them as text; the Flagged mark is `lib-viper`'s, shared with Bills.

use lib_core::TransactionStatus;
use lib_viper::display::{StatusGlyphs, flag_glyph};

use crate::msg;

/// The status glyph: Open ○, Cleared ◐, Reconciled ● in the unicode style. The ascii fallback keeps
/// the Display preview's own mapping for the glyphs they share (◐ is `o`, ● is `x`) and gives Open
/// the remaining mark, `-`.
pub fn status_glyph(status: &TransactionStatus, style: StatusGlyphs) -> &'static str {
    match (status, style) {
        (TransactionStatus::Open, StatusGlyphs::Unicode) => "\u{25cb}",
        (TransactionStatus::Cleared, StatusGlyphs::Unicode) => "\u{25d0}",
        (TransactionStatus::Reconciled, StatusGlyphs::Unicode) => "\u{25cf}",
        (TransactionStatus::Open, StatusGlyphs::AsciiFallback) => "-",
        (TransactionStatus::Cleared, StatusGlyphs::AsciiFallback) => "o",
        (TransactionStatus::Reconciled, StatusGlyphs::AsciiFallback) => "x",
    }
}

/// The status line's glyph legend, in the chosen style:
/// `○ open · ◐ cleared · ● reconciled · ⚑ flagged`.
pub fn status_legend(style: StatusGlyphs) -> String {
    let statuses = [
        (TransactionStatus::Open, msg::transactions_legend_open()),
        (
            TransactionStatus::Cleared,
            msg::transactions_legend_cleared(),
        ),
        (
            TransactionStatus::Reconciled,
            msg::transactions_legend_reconciled(),
        ),
    ];
    let mut parts: Vec<String> = statuses
        .iter()
        .map(|(status, name)| msg::transactions_legend_entry(status_glyph(status, style), name))
        .collect();
    parts.push(msg::transactions_legend_entry(
        flag_glyph(style),
        &msg::transactions_legend_flagged(),
    ));
    parts.join(&msg::transactions_legend_separator())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::test_support::locale;

    #[test]
    fn status_glyphs_match_the_transactions_bundle_in_unicode() {
        let unicode = StatusGlyphs::Unicode;
        assert_eq!(status_glyph(&TransactionStatus::Open, unicode), "\u{25cb}");
        assert_eq!(
            status_glyph(&TransactionStatus::Cleared, unicode),
            "\u{25d0}"
        );
        assert_eq!(
            status_glyph(&TransactionStatus::Reconciled, unicode),
            "\u{25cf}"
        );
        assert_eq!(flag_glyph(unicode), "\u{2691}");
    }

    #[test]
    fn the_ascii_fallback_keeps_the_display_previews_marks_for_shared_glyphs() {
        let ascii = StatusGlyphs::AsciiFallback;
        assert_eq!(status_glyph(&TransactionStatus::Cleared, ascii), "o");
        assert_eq!(status_glyph(&TransactionStatus::Reconciled, ascii), "x");
        assert_eq!(flag_glyph(ascii), "!");
        assert_eq!(status_glyph(&TransactionStatus::Open, ascii), "-");
        for style in StatusGlyphs::ALL {
            let glyphs = [
                status_glyph(&TransactionStatus::Open, style),
                status_glyph(&TransactionStatus::Cleared, style),
                status_glyph(&TransactionStatus::Reconciled, style),
                flag_glyph(style),
            ];
            let mut unique = glyphs.to_vec();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), 4, "every mark is distinct in {style:?}");
        }
    }

    #[test]
    fn the_legend_names_every_status_and_the_flag_in_the_chosen_style() {
        locale();
        assert_eq!(
            status_legend(StatusGlyphs::Unicode),
            "\u{25cb} open \u{b7} \u{25d0} cleared \u{b7} \u{25cf} reconciled \u{b7} \u{2691} flagged"
        );
        assert_eq!(
            status_legend(StatusGlyphs::AsciiFallback),
            "- open \u{b7} o cleared \u{b7} x reconciled \u{b7} ! flagged"
        );
    }
}
