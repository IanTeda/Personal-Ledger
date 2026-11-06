//! Each Locale's Currency: what a new Ledger's base Unit is prefilled with (#912). The Locale set
//! is closed, so every Locale has one and there is no further fallback to reach for.

use crate::{Locale, msg};

/// A Currency a Locale names as its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    Usd,
    Gbp,
    Aud,
}

impl Currency {
    /// The ISO 4217 code, lower case as a Unit's code is written.
    pub fn code(self) -> &'static str {
        match self {
            Currency::Usd => "usd",
            Currency::Gbp => "gbp",
            Currency::Aud => "aud",
        }
    }

    /// Its name in the Locale in effect, as a new base Unit is written with it.
    pub fn name(self) -> String {
        match self {
            Currency::Usd => msg::currency_usd(),
            Currency::Gbp => msg::currency_gbp(),
            Currency::Aud => msg::currency_aud(),
        }
    }
}

impl Locale {
    /// This Locale's Currency. The pseudo-Locale has none of its own, so it follows its fallback,
    /// `en-US`.
    pub fn currency(self) -> Currency {
        match self {
            Locale::EnUs | Locale::EnXa => Currency::Usd,
            Locale::EnGb => Currency::Gbp,
            Locale::EnAu => Currency::Aud,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::with_locale;

    #[test]
    fn en_us_names_the_us_dollar() {
        let currency = Locale::EnUs.currency();
        assert_eq!(currency.code(), "usd");
        assert_eq!(with_locale(Locale::EnUs, || currency.name()), "US Dollar");
    }

    #[test]
    fn en_gb_names_the_pound_sterling() {
        let currency = Locale::EnGb.currency();
        assert_eq!(currency.code(), "gbp");
        assert_eq!(
            with_locale(Locale::EnGb, || currency.name()),
            "British Pound"
        );
    }

    #[test]
    fn en_au_names_the_australian_dollar() {
        let currency = Locale::EnAu.currency();
        assert_eq!(currency.code(), "aud");
        assert_eq!(
            with_locale(Locale::EnAu, || currency.name()),
            "Australian Dollar"
        );
    }

    #[test]
    fn en_xa_follows_its_fallback_to_en_us() {
        assert_eq!(Locale::EnXa.currency(), Locale::EnUs.currency());
    }
}
