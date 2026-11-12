//! The tracing level Configuration sets, [`Levels`], is `lib-core`'s (ADR-0058): this turns it
//! into the `LevelFilter` the subscriber filters by. A function, not `From`, since the orphan
//! rule forbids implementing a foreign trait between two foreign types.

pub use lib_core::Levels;

use tracing::level_filters::LevelFilter;

/// The `LevelFilter` a configured level filters by, each to its namesake.
pub fn level_filter(level: Levels) -> LevelFilter {
    match level {
        Levels::OFF => LevelFilter::OFF,
        Levels::ERROR => LevelFilter::ERROR,
        Levels::WARN => LevelFilter::WARN,
        Levels::INFO => LevelFilter::INFO,
        Levels::DEBUG => LevelFilter::DEBUG,
        Levels::TRACE => LevelFilter::TRACE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_filters_by_its_namesake() {
        assert_eq!(
            [
                Levels::OFF,
                Levels::ERROR,
                Levels::WARN,
                Levels::INFO,
                Levels::DEBUG,
                Levels::TRACE,
            ]
            .map(level_filter),
            [
                LevelFilter::OFF,
                LevelFilter::ERROR,
                LevelFilter::WARN,
                LevelFilter::INFO,
                LevelFilter::DEBUG,
                LevelFilter::TRACE,
            ]
        );
    }
}
