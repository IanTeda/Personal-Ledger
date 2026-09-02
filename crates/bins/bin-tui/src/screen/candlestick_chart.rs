//! The candlestick chart feasibility demo (FC-TUI-002 continued): renders synthetic daily
//! OHLC price data via the third-party `chandelier` crate. Per ADR-0002
//! (`docs/adr/0002-ratatui-for-tui-charting.md`), this was prototyped first rather than
//! building on `Canvas` directly — it resolved cleanly against our `ratatui` 0.30.2 and
//! rendered correctly on the first attempt, so no custom fallback was needed.

use chandelier::{Candle, CandleSeries, CandlestickChart};
use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::{action::Action, screen::Screen};

/// Number of synthetic trading days to plot.
const DAY_COUNT: usize = 30;

/// Demonstrates a candlestick chart of synthetic daily OHLC price data, standing in for a
/// real investment's price history (see the "Personal Investors" future consideration in
/// `docs/product-requirements.md`).
pub struct CandlestickChartScreen {
    candles: Vec<Candle>,
}

impl CandlestickChartScreen {
    /// Builds the screen with deterministic dummy OHLC data — a small daily random walk.
    pub fn new() -> Self {
        let mut price = 100.0_f64;
        // A tiny xorshift PRNG: deterministic across runs/platforms, no `rand` dependency
        // needed for a feasibility demo.
        let mut seed: u64 = 42;
        let mut random_delta = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 400) as f64 / 100.0 - 2.0
        };

        let candles = (0..DAY_COUNT)
            .map(|_| {
                let open = price;
                let close = (open + random_delta()).max(1.0);
                let high = open.max(close) + random_delta().abs();
                let low = (open.min(close) - random_delta().abs()).max(0.5);
                price = close;
                Candle::new(open, high, low, close)
            })
            .collect();
        Self { candles }
    }
}

impl Screen for CandlestickChartScreen {
    fn update(&mut self, _action: &Action) {
        // Static dummy data — nothing to react to yet.
    }

    fn title(&self) -> &'static str {
        "Candlestick Chart"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let chart = CandlestickChart::new(CandleSeries::new(&self.candles))
            .block(Block::bordered().title(" Candlestick chart demo — FC-TUI-002 (dummy data) "));
        frame.render_widget(chart, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let screen = CandlestickChartScreen::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the candlestick chart should not error");
    }
}
