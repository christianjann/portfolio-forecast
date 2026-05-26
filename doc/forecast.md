# Portfolio Forecast — Algorithm & Functionality

## Overview

Portfolio Forecast reads a [Portfolio Performance](https://www.portfolio-performance.info/) `.portfolio` file, replays every transaction to compute a historical **Net Asset Value (NAV)** series, then projects that series 10 years into the future using a **time-weighted return rate (TTWROR)** combined with a **monthly capital-addition annuity**.

---

## File Loading Pipeline

```
.portfolio file / content URI
        │
        ▼
  format::load_bytes()
        │
        ├─ ZIP magic (PK\x03\x04)  → zip::load()   → prost decode → PClient
        ├─ Binary magic (PPPBV1)   → binary::load() → prost decode → PClient
        └─ "PORTFOLIO" prefix      → error (encrypted — not supported)
```

The internal data model (`PClient`) is the Protobuf representation used by Portfolio Performance (`name.abuchen.portfolio` package), decoded with `prost`.

---

## Historical NAV Series (`compute_nav_series`)

`common/src/analysis/balance.rs`

### Data encoding

| Field | Storage unit | Conversion |
|-------|-------------|------------|
| `amount` | integer cents | `÷ 100` → currency units |
| `shares` | ×10⁸ integer | `÷ 1e8` → number of shares |
| `price.close` | ×10⁸ integer | `÷ 1e8` → currency units per share |

### Algorithm

1. **Build a security price map** — for every security, take the last available `prices` entry or the `latest` quote snapshot and store `price (currency units)` keyed by security UUID.
2. **Sort transactions** by UNIX timestamp (ascending).
3. **Replay transactions** in order, maintaining two running accumulators:
   - `cash: HashMap<account_uuid, f64>` — running cash balance per account.
   - `positions: HashMap<portfolio_uuid, HashMap<security_uuid, f64>>` — share counts per security per portfolio.
4. **Apply each transaction** according to its type:

| Type code | Name | Cash effect | Share effect |
|-----------|------|-------------|--------------|
| 0 | PURCHASE | `account − amount` | `portfolio + shares` |
| 1 | SALE | `account + amount` | `portfolio − shares` |
| 2 | INBOUND_DELIVERY | — | `portfolio + shares` |
| 3 | OUTBOUND_DELIVERY | — | `portfolio − shares` |
| 4 | SECURITY_TRANSFER | — | `src_portfolio − shares`, `dst_portfolio + shares` |
| 5 | CASH_TRANSFER | `src_account − amount` | — |
| 5 | CASH_TRANSFER | `dst_account + amount` | — |
| 6 | DEPOSIT | `account + amount` | — |
| 7 | REMOVAL | `account − amount` | — |
| 8 | DIVIDEND | `account + amount` | — |
| 9 | INTEREST | `account + amount` | — |
| 10 | INTEREST_CHARGE | `account − amount` | — |
| 11 | TAX | `account − amount` | — |
| 12 | TAX_REFUND | `account + amount` | — |
| 13 | FEE | `account − amount` | — |
| 14 | FEE_REFUND | `account + amount` | — |

5. **After each transaction**, compute:

$$\text{NAV} = \sum_{\text{accounts}} \text{cash} + \sum_{\text{positions}} \text{shares} \times \text{latest\_price}$$

6. Emit a `NavPoint { date, nav }` for:
   - **each transaction** (post-transaction NAV at the transaction date), and
   - **each security price-change date** that falls between transactions (so that market moves are reflected even when no transaction occurs on that day).

   Price-change dates that coincide with a transaction day emit only the transaction point (no duplicate).

> **Note:** Prices are looked up using the last known price on or before each transaction date (step-function / "previous tick"), matching the same approach used by the TWR computation.

---

## Forecast (`compute_forecast` + `compute_twr_rate`)

`common/src/views/portfolio_screen.rs`

The forecast is a 10-year (120-month) forward projection starting from the last historical NAV point.

### Step 1 — Annual TWR Rate (`compute_twr_rate`)

The function computes a **time-weighted rate of return** using the Modified Dietz / sub-period chain approach:

1. Build a full price history map: `security_uuid → Vec<(epoch_day, price)>`, sorted chronologically.
2. Sort all transactions by timestamp.
3. Determine the time horizon:
   - `first_secs` — timestamp of the first transaction.
   - `end_secs` — the later of the last transaction timestamp or the last available price date across all securities. This extends the horizon when prices are available beyond the last cash-flow event.
   - `total_years = (end_secs − first_secs) / 86400 / 365.25` (minimum 0.5 years).
4. Replay transactions while computing sub-period returns around every **external cash-flow event** — DEPOSIT (6), REMOVAL (7), INBOUND_DELIVERY (2), OUTBOUND_DELIVERY (3):
   - Compute `nav_before` (cash + market value of positions at that day's prices) *before* the external flow is applied.
   - If a previous sub-period start value `psv > 0`, multiply into the running product: `twr_product *= nav_before / psv`.
   - After applying the flow, record the new `nav_after` as `period_start_val`.

   > INBOUND/OUTBOUND_DELIVERY are treated as external flows because securities enter or leave the portfolio without a corresponding cash transaction — identical to how Portfolio Performance handles deposit-in-kind events.
5. At the end, compute a final sub-period return from `period_start_val` to the portfolio's current value at the latest price day.
6. Convert the total return product to an annualised rate:

$$r_{\text{annual}} = \left(\prod_i \frac{\text{NAV\_before}_i}{\text{NAV\_after}_{i-1}}\right)^{1/\text{total\_years}} - 1$$

Clamped to the range **[−50 %, +150 %]**. Falls back to **7 % p.a.** if there are fewer than two transactions or if the total return product is non-positive.

`price_at` uses the **last known price on or before** a given epoch day (step-function / "previous tick" lookup).

### Step 2 — Average Monthly Net Capital Addition (`avg_monthly`)

Over a rolling window of **up to 60 months** ending on the last NAV date (or all available months if history is shorter):

$$\text{avg\_monthly} = \frac{\text{inflow} - \text{outflow}}{\text{months\_used}}$$

where:
- **Inflow** — sum of amounts for DEPOSIT (6), DIVIDEND (8), INTEREST (9), INBOUND_DELIVERY (2).
- **Outflow** — sum of amounts for REMOVAL (7), OUTBOUND_DELIVERY (3).

### Step 3 — 10-Year Projection

For each month $k = 1 \ldots 120$ from the last NAV date:

$$\text{NAV}_{k} = \underbrace{\text{NAV}_{\text{last}} \cdot (1 + r)^{k/12}}_{\text{growth}} + \underbrace{\text{avg\_monthly} \cdot \frac{(1 + r_m)^k - 1}{r_m}}_{\text{annuity}}$$

where $r$ is the annual TWR rate and $r_m = (1+r)^{1/12} - 1$ is the equivalent monthly rate.

When $|r_m| \leq 10^{-10}$ (i.e. effectively zero growth), the annuity degenerates to the linear approximation $\text{avg\_monthly} \cdot k$.

---

## Chart & UI

The chart (`paint_nav_chart`) renders:

- **Historical NAV** — blue area fill + 1.5 px stroke line.
- **Forecast** — orange area fill + 1.5 px stroke line, beginning at the last historical point.
- A vertical **"today" divider** at the boundary between history and forecast.
- A **forecast legend** (top-right corner) showing:
  - `TTWROR` — annualised TWR rate in % p.a.
  - `Monthly` — average monthly net capital addition.
  - `Period` — number of months used to compute the average addition.

A touch/mouse **crosshair** snaps to the nearest data point on either the historical or forecast series and shows a tooltip with the date and NAV value.

**Milestone markers** are rendered on the forecast portion of the chart as filled amber circles, each labelled with the projected date and the milestone amount. The set of milestones is:

| Milestone |
|-----------|
| 10 k |
| 100 k |
| 500 k |
| 1 M |
| 2 M |
| 5 M |
| 10 M |

A marker is shown only when **both** conditions hold:
1. The current NAV is still **below** the milestone value (not yet reached).
2. The milestone value falls **within the visible y-axis range** of the chart (i.e. the forecast reaches it within the 10-year window).

The crossing date is found by `find_forecast_crossing`, which does a linear interpolation between the two monthly forecast points that straddle the target value. The label box is placed to the right of the dot (or to the left when near the right chart edge) and shows the month (`YYYY-MM`) on the first line and the formatted amount on the second line.

Axis ticks are computed by `nice_ticks` (Y-axis, decimal rounding) and `time_ticks` (X-axis, calendar month snapping at 1 / 3 / 6 / 12 / 24 / 60 / 120-month intervals).

---

## Platform Notes

| Platform | File opening | Binary reading |
|----------|-------------|----------------|
| Android | SAF content URI via JNI (`GpuiContentReader`) | `load_from_bytes` |
| iOS | Real filesystem path | `std::fs::read` + `load_from_bytes` |
| Desktop | `rfd` / GPUI file selector | `load_file` |

Encrypted `.portfolio` files (magic `PORTFOLIO`) are not supported; users must export as unencrypted from Portfolio Performance.
