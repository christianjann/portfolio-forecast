use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;

use crate::proto::{PClient, PTransaction};

/// A single NAV (net asset value) observation at a given date.
#[derive(Clone, Debug)]
pub struct NavPoint {
    pub date: NaiveDate,
    /// NAV in base-currency units (e.g. EUR).
    pub nav: f64,
}

/// Replay all transactions chronologically and emit a NavPoint timeline.
///
/// Points are emitted at two kinds of dates:
/// - **Transaction dates** — after each transaction is applied.
/// - **Price-change dates** — every date a security price changes, so that
///   market moves between transactions are reflected in the series (giving a
///   smooth curve rather than a staircase when transactions are infrequent).
///
/// Amounts are stored as integer cents (÷100 for currency units).
/// Shares and prices are stored as ×1e8 integers (÷1e8 for the real value).
pub fn compute_nav_series(client: &PClient) -> Vec<NavPoint> {
    // Build security UUID → sorted price history [(epoch_day, price)].
    let mut price_map: HashMap<&str, Vec<(i64, f64)>> = HashMap::new();
    for sec in &client.securities {
        let mut prices: Vec<(i64, f64)> = sec
            .prices
            .iter()
            .map(|p| (p.date, p.close as f64 / 1e8))
            .collect();
        if let Some(latest) = &sec.latest {
            prices.push((latest.date, latest.close as f64 / 1e8));
        }
        prices.sort_unstable_by_key(|p| p.0);
        prices.dedup_by_key(|p| p.0);
        if !prices.is_empty() {
            price_map.insert(sec.uuid.as_str(), prices);
        }
    }

    // Sort transactions by timestamp (seconds since epoch).
    let mut sorted: Vec<&PTransaction> = client.transactions.iter().collect();
    sorted.sort_by_key(|t| t.date.as_ref().map(|d| d.seconds).unwrap_or(0));

    if sorted.is_empty() {
        return vec![];
    }

    let first_epoch_day = sorted[0]
        .date.as_ref().map(|d| d.seconds / 86_400).unwrap_or(0);

    // Collect epoch-days that already have a transaction — price-date points
    // on those days would duplicate the transaction-date point.
    let tx_epoch_days: HashSet<i64> = sorted.iter()
        .filter_map(|t| t.date.as_ref().map(|d| d.seconds / 86_400))
        .collect();

    // Price-only days: price-change dates that fall on or after the first
    // transaction and don't coincide with a transaction day.
    let price_only_days: Vec<i64> = {
        let days: HashSet<i64> = price_map
            .values()
            .flat_map(|prices| prices.iter().map(|(day, _)| *day))
            .filter(|&day| day >= first_epoch_day && !tx_epoch_days.contains(&day))
            .collect();
        let mut v: Vec<i64> = days.into_iter().collect();
        v.sort_unstable();
        v
    };

    // Portfolio state.
    let mut cash: HashMap<String, f64> = HashMap::new();
    let mut positions: HashMap<String, HashMap<String, f64>> = HashMap::new();

    let mut points: Vec<NavPoint> = Vec::new();
    let mut pd_idx = 0usize;

    for t in &sorted {
        let date_sec = match &t.date {
            Some(ts) => ts.seconds,
            None => continue,
        };
        let epoch_day = date_sec / 86_400;
        let amount = t.amount as f64 / 100.0;
        let shares = t.shares.unwrap_or(0) as f64 / 1e8;

        // Emit price-date points that fall strictly before this transaction.
        while pd_idx < price_only_days.len() && price_only_days[pd_idx] < epoch_day {
            let day = price_only_days[pd_idx];
            pd_idx += 1;
            if let Some(dt) = chrono::DateTime::from_timestamp(day * 86_400, 0) {
                points.push(NavPoint { date: dt.date_naive(), nav: nav_at(&cash, &positions, &price_map, day) });
            }
        }

        // Apply the transaction.
        match t.r#type {
            // DEPOSIT (6), INTEREST (9), DIVIDEND (8), TAX_REFUND (12), FEE_REFUND (14)
            6 | 9 | 8 | 12 | 14 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() += amount;
                }
            }
            // REMOVAL (7), INTEREST_CHARGE (10), TAX (11), FEE (13)
            7 | 10 | 11 | 13 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() -= amount;
                }
            }
            // PURCHASE (0): debit cash, add shares
            0 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() -= amount;
                }
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions
                        .entry(port.clone())
                        .or_default()
                        .entry(sec.clone())
                        .or_default() += shares;
                }
            }
            // SALE (1): credit cash, remove shares
            1 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() += amount;
                }
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions
                        .entry(port.clone())
                        .or_default()
                        .entry(sec.clone())
                        .or_default() -= shares;
                }
            }
            // INBOUND_DELIVERY (2): add shares, no cash effect
            2 => {
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions
                        .entry(port.clone())
                        .or_default()
                        .entry(sec.clone())
                        .or_default() += shares;
                }
            }
            // OUTBOUND_DELIVERY (3): remove shares, no cash effect
            3 => {
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions
                        .entry(port.clone())
                        .or_default()
                        .entry(sec.clone())
                        .or_default() -= shares;
                }
            }
            // CASH_TRANSFER (5): move cash between accounts
            5 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() -= amount;
                }
                if let Some(other) = &t.other_account {
                    *cash.entry(other.clone()).or_default() += amount;
                }
            }
            // SECURITY_TRANSFER (4): move shares between portfolios
            4 => {
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions
                        .entry(port.clone())
                        .or_default()
                        .entry(sec.clone())
                        .or_default() -= shares;
                }
                if let (Some(other_port), Some(sec)) = (&t.other_portfolio, &t.security) {
                    *positions
                        .entry(other_port.clone())
                        .or_default()
                        .entry(sec.clone())
                        .or_default() += shares;
                }
            }
            _ => {}
        }

        // Emit transaction-date point (post-transaction NAV).
        if let Some(dt) = chrono::DateTime::from_timestamp(date_sec, 0) {
            points.push(NavPoint {
                date: dt.date_naive(),
                nav:  nav_at(&cash, &positions, &price_map, epoch_day),
            });
        }
    }

    // Emit any remaining price-date points after the last transaction.
    while pd_idx < price_only_days.len() {
        let day = price_only_days[pd_idx];
        pd_idx += 1;
        if let Some(dt) = chrono::DateTime::from_timestamp(day * 86_400, 0) {
            points.push(NavPoint { date: dt.date_naive(), nav: nav_at(&cash, &positions, &price_map, day) });
        }
    }

    points
}

/// Compute NAV at `epoch_day` given current cash and positions.
fn nav_at(
    cash: &HashMap<String, f64>,
    positions: &HashMap<String, HashMap<String, f64>>,
    price_map: &HashMap<&str, Vec<(i64, f64)>>,
    epoch_day: i64,
) -> f64 {
    let total_cash: f64 = cash.values().sum();
    let total_sec: f64 = positions
        .values()
        .flat_map(|p| p.iter())
        .filter_map(|(sec_uuid, &shs)| {
            price_at(price_map.get(sec_uuid.as_str())?, epoch_day).map(|price| shs * price)
        })
        .sum();
    total_cash + total_sec
}

/// Look up the last known price on or before `epoch_day` (step-function lookup).
fn price_at(prices: &[(i64, f64)], epoch_day: i64) -> Option<f64> {
    if prices.is_empty() {
        return None;
    }
    let idx = prices.partition_point(|p| p.0 <= epoch_day);
    Some(if idx == 0 { prices[0].1 } else { prices[idx - 1].1 })
}
