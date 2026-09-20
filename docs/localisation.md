# Localisation

Personal Ledger can show its screens, and format your numbers, dates and money, the way people in your part of the world expect. This page explains what you get, how it picks your region, and how to change it.

> **Heads up:** this is still being built. This page describes how localisation will work once it lands, so a few things here won't be in your copy of the app yet.

## What you get

Your **locale** is your language plus your region, for example `en-AU` for English as spoken in Australia. Personal Ledger starts with three:

- **`en-US`**: American English (color, ZIP code).
- **`en-GB`**: British English (colour, postcode).
- **`en-AU`**: Australian English.

Your locale changes two things, and they always match each other:

- **The wording and spelling** on the screens, in both the desktop app and the terminal app.
- **How numbers, dates and money are written.** Dates come out day-first in Australia and the UK and month-first in the US. Amounts use your region's separators and put the currency symbol where you'd expect it. An Australian dollar account shown to an American reader might read `A$1,234.56`, so it can't be mistaken for US dollars.

## Typing dates

When you type a date, use whichever order is normal where you live, so `3/9/26` means the 3rd of September in Australia and the 9th of March in the US. Two things always work, whatever your locale:

- **ISO dates**, written year first: `2026-09-03`. They can't be misread, so they're a good habit if you move between regions.
- **Plain words**: `today`, `yesterday` and `tomorrow`.

## Which locale do I get?

Personal Ledger looks at your computer's own language and region settings and uses those. If your computer is set to something Personal Ledger doesn't have yet (say, French or New Zealand English), you'll get American English, which is the fallback for everything.

## Changing it

You change your locale in the config file, then restart Personal Ledger. There's no switch inside the app on purpose: your locale belongs to the computer you're using, not to your ledger, so a laptop and a desktop can use different ones without fighting each other over a synced setting.

The simplest way is to add a `locale` line to your config file (see [Configuration](configuration.md) for where that lives):

```ini
[Personal-Ledger]
locale = en-GB
```

You can also set it for a single run, which is handy for trying things out:

```sh
personal_ledger --locale en-GB
```

Or with an environment variable:

```sh
PERSONAL_LEDGER_PERSONAL_LEDGER__LOCALE=en-GB
```

If you set it more than one way, the most specific wins: the command-line flag beats the environment variable, which beats the config file, which beats your computer's settings. If you make a typo (`en_GB!!`), Personal Ledger stops at startup and tells you, rather than quietly picking something else.

The Display page in Settings shows which locale is in use and where it came from (your system, or the config), so you can see at a glance why you're getting what you're getting. It just shows it: you change it with the steps above.

## Your date style

Separately, Settings → Display lets you pick how dates look: short, medium, long, or ISO. Leave it alone and it follows your locale. Pick ISO and dates always show as `2026-09-03`, whatever your locale says. This one changes straight away, and it syncs across your devices like your other display choices.

## Good to know

- **Your own words stay as you typed them.** Account, payee, category and tag names are your data, so they're never translated or changed.
- **Commands stay in English.** The palette commands you type (like `accounts new`) are the same in every locale, though the descriptions next to them follow your locale.
- **Only English for now.** The three locales above are the ones planned. More languages aren't on the list yet.
- **Spotted something that looks wrong** or hasn't been translated? Please [open an issue](https://github.com/IanTeda/Personal-Ledger/issues) and say which screen and which locale.

## For the curious

There's also a hidden fourth locale, `en-XA`, that isn't offered to anyone in Settings. It's a "pseudo" locale that squiggles every letter with accents and stretches the text, which makes it easy to spot any wording that hasn't been converted yet and any screen where longer text won't fit. You can turn it on with `--locale en-XA` if you want to see what it does.

## For developers

Want to know how it all works underneath, or how to add a screen's text? See the [Localisation design](localisation-design.md).
