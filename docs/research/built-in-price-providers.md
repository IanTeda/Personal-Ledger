# Research: what the built-in price providers offer today

**Question.** Ticket #1368 (child of the map #1367, "Units price themselves from their Price Sources"): for the three built-in Price Sources, CoinGecko, Yahoo Finance and Frankfurter, what is the current address and single-price request, does it need a key, what are the free tier's rate limits and terms for personal use from a desktop app, can one request price several symbols, is there a history endpoint (grain, and how far back on the free tier), and how are symbols named? And do the `const` endpoints in `crates/libs/lib-units/src/entity/price_source.rs` (on `concept`) still work?

**Method.** Each provider's own documentation and terms were read on 23 November 2026, and every endpoint below was called live from Australia the same day with `curl`, so "observed" means a request made for this note, not a secondary write-up. Yahoo Finance publishes no documentation for its chart API; for Yahoo, the observed behaviour is the primary source, and community reports are cited and marked as such.

## Summary

| | CoinGecko | Yahoo Finance | Frankfurter |
| --- | --- | --- | --- |
| Current `const` works? | Yes, as written | Only with a browser-like `User-Agent`; the default `curl` agent and an empty one get `429` | Only through a `301` redirect to `api.frankfurter.dev/v1`; the domain has moved |
| Key | None (keyless), or a free Demo key in `x-cg-demo-api-key` | None; no crumb needed for `/v8/finance/chart` | None |
| Free rate limit | Keyless about 5 calls/min shared per IP; Demo 100 calls/min and 10,000 calls/month | Unpublished; observed `429`s | "No quotas", rate-limited only against abuse |
| Personal use terms | Keyless "designed for quick testing", not production; attribution required; Demo has no commercial licence | Yahoo's ToS forbid collecting data "using any automated means"; there is no licensed free API | Free, including commercial use; open source and self-hostable |
| Batching | Yes: `ids` comma list (up to 515) and `vs_currencies` comma list | Chart: one symbol per request; `/v7/finance/spark` takes several symbols without a crumb (observed) | Yes: several quote currencies per request |
| History | Daily at 00:00 UTC, past 365 days only on keyless and Demo | Daily, weekly or monthly OHLC plus adjusted close, back to the first trade date (CBA.AX 1991, BTC-AUD 2014) | Daily business-day rates: v1 (ECB) from 4 January 1999; v2 back to 1948 for some currencies |
| Symbols | API ids (`bitcoin`), not tickers (`BTC`) | Yahoo tickers with exchange suffix (`CBA.AX`), crypto `BTC-AUD`, FX `AUDUSD=X` | ISO 4217 codes (`AUD`) |

## CoinGecko

### Address and the request for one price

The keyless and Demo base address is `https://api.coingecko.com/api/v3`; one price is `GET /simple/price?ids=bitcoin&vs_currencies=aud` (CoinGecko, "Coin Price by IDs", https://docs.coingecko.com/v3.0.1/reference/simple-price). Paid plans use `https://pro-api.coingecko.com/api/v3` with `x-cg-pro-api-key` (same docs family; not exercised here). The keyless page lists the same base address (https://docs.coingecko.com/docs/keyless-public-api.md).

The `const` in `price_source.rs` (`https://api.coingecko.com/api/v3` + `/simple/price?ids={symbol}&vs_currencies={quote}`, price at `/{symbol}/{quote}`) works as written. Observed: `200`, body `{"bitcoin":{"usd":61062,"aud":91987},"ethereum":{...}}`. One caveat for the price path: CoinGecko returns the `vs_currencies` key in lower case (`aud`), so `{quote}` must be lower-cased when it fills both the request and the path.

### Key and rate limits

No key is needed. The keyless API's page says "All keyless requests share an IP-based rate pool" of about 5 calls/min, varying with server load, and to back off exponentially on `429` (https://docs.coingecko.com/docs/keyless-public-api.md). Observed: after about ten calls in a minute, a `429` with `retry-after: 60`. Other CoinGecko pages and search snippets give 10–30 calls/min for keyless; the keyless page itself is the most specific and was used here.

The free Demo plan gives "100 requests/minute" and "10k call credits/month", one key per account, sent as the header `x-cg-demo-api-key` or the query parameter `x_cg_demo_api_key` on the same `api.coingecko.com` address (pricing, https://www.coingecko.com/en/api/pricing.md; simple price docs above). Every request counts against the minute limit, whatever its status (CoinGecko errors docs, via https://docs.coingecko.com/docs/errors-and-rate-limits; not re-read in full).

A Demo key is per person, so in Personal Ledger it would be the Price Source's `ApiKey` auth, entered by the person, not shipped in the app.

### Terms for personal use

The keyless page says it is "designed for quick testing" and fits prototyping, open-source projects and education, and advises against "production environments, scheduled polling, and high-frequency updates"; displaying keyless data requires "appropriate attribution" (https://docs.coingecko.com/docs/keyless-public-api.md). The pricing page lists the Demo plan's licence as "Not included" and "Attribution required" (https://www.coingecko.com/en/api/pricing.md).

The API Terms require "Powered by CoinGecko" displayed prominently (section 4.3), forbid re-distributing or syndicating access to the API (4.1.6), and on storage say "We do not encourage caching or storage of Data", that a cache should be refreshed "at least every 24 hours", and that you may not "store ... any Data" except as the terms permit (6.1, 6.2) (https://www.coingecko.com/en/api_terms). Read strictly, keeping a Unit's fetched price history in a Ledger sits uneasily with section 6; this is a terms question for the map, not a technical one. The terms do not address end-user desktop apps or per-user keys.

### Batching

Yes. `ids` takes a comma list, "Maximum of 515 IDs per request", and `vs_currencies` a comma list (simple price docs above). Observed: `ids=bitcoin,ethereum&vs_currencies=aud,usd` answered all four prices in one call. With the keyless limit near 5 calls/min, batching every crypto Unit into one request per refresh matters.

### History

`GET /coins/{id}/market_chart?vs_currency=aud&days=N` returns `prices`, `market_caps` and `total_volumes` as `[ms, value]` pairs; above 90 days the grain is daily, stamped 00:00 UTC, and "The last completed UTC day (00:00) is available after 00:35 UTC the next day" (https://docs.coingecko.com/v3.0.1/reference/coins-id-market-chart). These are single prices per day, not OHLC; `/coins/{id}/ohlc` gives OHLC candles (not exercised: it hit the rate limit). `/coins/{id}/history?date=dd-mm-yyyy` gives one day, and `/coins/{id}/market_chart/range` a from–to range.

How far back: "Historical data via the Demo API is restricted to the past 365 days" (market chart docs above); pricing lists "Daily historical data: 1 year" for Demo. Observed for keyless: `days=365` returns `200`; `days=400`, `history?date=01-01-2020` and a 2020 `market_chart/range` all return `401` with error code `10012`, "Public API users are limited to querying historical data within the past 365 days". Full history needs the paid Analyst plan or above. So a backfill floored at an Account's Opened date more than a year ago cannot be met from free CoinGecko.

### Symbols

CoinGecko prices by its own API id (`bitcoin`, `ethereum`), not the ticker (`BTC`). `simple/price` also accepts `symbols` and `names`, with "priority ... `ids` > `names` > `symbols`" (simple price docs above), but tickers are not unique: observed, `GET /search?query=btc` returns `bitcoin` first, then `bitget-wrapped-btc` and others. The id is the safe link; `/search` (or `/coins/list`) can map a ticker to ids for a picker.

## Yahoo Finance

Yahoo publishes no documentation for the `query1.finance.yahoo.com` endpoints: they are the private API behind the Yahoo Finance website. Everything in this section is observed behaviour on 23 November 2026 unless cited.

### Address and the request for one price

`GET https://query1.finance.yahoo.com/v8/finance/chart/{symbol}` (`query2.` also answers). The current price is at `chart.result[0].meta.regularMarketPrice`, with `meta.currency` giving the quote currency.

The `const` (`https://query1.finance.yahoo.com` + `/v8/finance/chart/{symbol}`, price at `/chart/result/0/meta/regularMarketPrice`) works, but only with a browser-like `User-Agent`. Observed on the same URL: `User-Agent: Mozilla/5.0` gave `200`; an empty agent gave `429 Too Many Requests` and `curl`'s default agent gave `429 Edge: Too Many Requests`. The chart endpoint did not need a cookie or crumb. So the built-in needs to send a browser-like `User-Agent` (an `Endpoint` today has no headers besides auth), and Yahoo's `regularMarketPrice` is in `meta.currency`, so the Unit's quote currency is fixed by the listing rather than chosen by `{quote}`.

Community reports agree: a January 2026 crate README says `/v8/finance/chart` needs no authentication while `quoteSummary` needs cookie and crumb (https://docs.rs/crate/digdigdig3/0.2.3/source/src/l1/free/yahoo/README.md), and `yfinance` issues show `429`s during its cookie and crumb step (https://github.com/ranaroussi/yfinance/issues/2480). These are not Yahoo's word and can change without notice.

### Key and rate limits

No key. Yahoo publishes no rate limit for these endpoints; the general Yahoo API terms say "Yahoo's APIs may be subject to rate limits at Yahoo's absolute and sole discretion" (https://legal.yahoo.com/us/en/yahoo/terms/product-atos/apiforydn/index.html). Observed: `429`s keyed at least on the `User-Agent`; a handful of calls a minute with a browser agent were not throttled.

### Terms for personal use

There is no licensed free Yahoo Finance API. Yahoo's Terms of Service forbid you to "access or collect data, or attempt to access or collect data, from our Services using any automated means" (section 2(d)(9)), "high volume activity without our prior written consent" (2(d)(13)) and, "Unless otherwise expressly stated", reusing the Services "for any commercial purpose" (2(e)) (https://legal.yahoo.com/us/en/yahoo/terms/otos/index.html). Yahoo Finance itself is provided by Yahoo Finance LLC under the same terms (14.2). The Yahoo API terms, which may or may not cover these endpoints, forbid deriving income from the APIs (https://legal.yahoo.com/us/en/yahoo/terms/product-atos/apiforydn/index.html). Personal, non-commercial use from a desktop app is therefore tolerated in practice by Yahoo, not permitted on paper: the risk is breakage and blocking, and the person, not Personal Ledger, makes the request.

### Batching

The chart endpoint takes one symbol. Observed: `GET /v7/finance/spark?symbols=CBA.AX,VAS.AX,BTC-USD&range=1d&interval=1d` answered all three symbols in one `200` without a crumb, each with the same `meta` block. `GET /v7/finance/quote?symbols=...`, the obvious batch quote, returned `401 Unauthorized` ("User is unable to access this feature"): it needs the crumb.

### History

The chart endpoint is also the history endpoint: `?period1={unix}&period2={unix}&interval=1d` (or `1wk`, `1mo`) returns `timestamp[]` with `indicators.quote[0]` holding `open`, `high`, `low`, `close` and `volume`, and `indicators.adjclose`. `range=` takes `1d`, `5d`, `1mo`, `3mo`, `6mo`, `1y`, `2y`, `5y`, `10y`, `ytd` and `max` (from `meta.validRanges`), and `events=div` adds dividends.

How far back: to the listing's first trade date, with no free-tier cap observed. Observed: `VAS.AX` daily from 1 May 2009 (4,446 rows), `CBA.AX` weekly from 1991, and `BTC-AUD` daily from 17 September 2014 (4,451 rows). One trap: `range=max&interval=1d` silently came back monthly (`meta.dataGranularity: "1mo"`, 212 rows); asking with `period1=0&period2=...` and `interval=1d` gave daily rows. Use `period1`/`period2`, never `range=max`, for a daily backfill.

### Symbols

Yahoo tickers with an exchange suffix: ASX listings end `.AX` (`CBA.AX`, `VAS.AX`; observed `meta.exchangeName: "ASX"`, `currency: "AUD"`). Crypto is `BASE-QUOTE` (`BTC-USD`, `BTC-AUD`), and currency pairs are `AUDUSD=X` (observed `instrumentType: "CURRENCY"`). Yahoo has no published suffix list; its help page on exchanges is the closest first-party source and was not read for this note.

## Frankfurter

### Address and the request for one price

Frankfurter has moved to `api.frankfurter.dev` and now has two versions. v2 is current: `https://api.frankfurter.dev/v2/rates?base=USD&quotes=AUD` returns `[{"date":"2026-11-23","base":"USD","quote":"AUD","rate":1.5005}]`, and the single pair `GET /v2/rate/USD/AUD` returns `{"date":...,"base":"USD","quote":"AUD","rate":1.5005}` (observed; https://frankfurter.dev/). v1 "is deprecated in favor of v2, but will continue to work" at `https://api.frankfurter.dev/v1/`, keeping the old shape: `GET /v1/latest?from=USD&to=AUD` returns `{"amount":1.0,"base":"USD","date":"2026-11-20","rates":{"AUD":1.5002}}` (https://frankfurter.dev/v1/; observed).

The `const` (`https://api.frankfurter.app` + `/latest?from={symbol}&to={quote}`, price at `/rates/{quote}`) still answers, but only through a redirect: observed `301 Moved Permanently` to `https://api.frankfurter.dev/v1/latest?from=USD&to=AUD`. Neither docs page mentions `api.frankfurter.app`, so how long the redirect lasts is unknown. The fix is to point the `const` at `https://api.frankfurter.dev/v1` (same request and price path) or move to v2 (`/v2/rate/{symbol}/{quote}`, price at `/rate`).

Note the dates: v1 is ECB-only and updated "daily around 16:00 CET", so on a Monday morning in Australia it answered with Friday's rate (`2026-11-20`); v2 blends 112 sources and "update[s] throughout the day", answering with today's date (`2026-11-23`).

### Key and rate limits

"You don't need an API key." "There are no quotas. Requests are rate-limited to prevent abuse, but there are no monthly or daily caps" (https://frankfurter.dev/). No `429` was seen.

### Terms for personal use

Free, including for commercial use and in paid products; the figures each upstream provider publishes stay subject to that provider's terms; the project is open source and can be self-hosted (https://frankfurter.dev/). It suits a desktop app without caveats.

### Batching

Yes for quote currencies, one base per request: v1 `?from=USD&to=AUD,EUR` (observed, both in `rates`), v2 `?base=USD&quotes=AUD,EUR` (https://frankfurter.dev/). Several bases need several requests, or one request in a common base and cross-rates computed locally.

### History

v1: `/v1/{YYYY-MM-DD}` for one day, `/v1/{start}..{end}` for a range, `/v1/{start}..` to today, all with `from`/`to` (https://frankfurter.dev/v1/). Grain is daily, business days only (ECB reference rates, no weekends or holidays); a range snaps its start back to the previous business day (observed: `2024-01-01..` began `2023-12-29`). Earliest date observed: 4 January 1999 (`200`); `1990-01-02` returned `404`. The docs do not state the earliest date.

v2: `/v2/rates?from={date}&to={date}&quotes=...` for a period (https://frankfurter.dev/). "Daily history goes back to 1948" for the US dollar, but "most currencies start in the 1990s or later" (https://frankfurter.dev/). Observed: AUD to USD from 4 January 1971; a 1960 USD to AUD request returned an empty array, not an error. Single rates per day, not OHLC; there is no free-tier cap.

### Symbols

ISO 4217 currency codes; v2 covers "219 currencies ... and 39 other assets" from 112 sources (https://frankfurter.dev/), v1 the ECB's set (about 30; `GET /v1/currencies` lists them). Case does not matter for v2 (the docs use `quotes=usd`); responses use upper case.

## What this means for the built-in table

- CoinGecko's `const` works, but the keyless pool (about 5 calls/min per IP) is too thin for Every 15 minutes across several Units unless they are batched into one request; a person's Demo key raises it to 100/min. Free history stops at 365 days, so backfill floored at an older Opened date needs another source (Yahoo's `BTC-AUD` goes back to 2014) or Manual entry.
- Yahoo's `const` works only with a browser-like `User-Agent`, which the `Endpoint` type cannot express today. It gives the deepest free history (daily OHLC to listing) but is unofficial and against Yahoo's ToS on paper.
- Frankfurter's `const` relies on a `301` from a domain its docs no longer mention; it should move to `api.frankfurter.dev` (v1 to keep the shape, or v2 for same-day rates and more currencies).

## Not verified

- CoinGecko's keyless limit: the keyless page says about 5 calls/min; other CoinGecko pages and summaries say 10–30. Observed throttling after about ten calls is consistent with the low end.
- CoinGecko `/coins/{id}/ohlc` on keyless: the probe hit `429` before an answer, so its grain and range were not seen.
- Yahoo's rate limits and whether the `User-Agent` check or the chart endpoint's crumb-free access will last: undocumented, observed once from one Australian IP.
- Whether the Yahoo API terms (`apiforydn`) cover the Finance chart endpoints; the page does not mention Yahoo Finance.
- How long `api.frankfurter.app` will keep redirecting.
- Frankfurter v1's earliest date is observed (1999-01-04), not documented.
