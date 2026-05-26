#!/usr/bin/env python3
"""
Generate example-portfolio.xml — an anonymous Portfolio Performance XML (v60)
file for verifying TTWROR and monthly-capital-addition calculations.

Scenario
--------
  Security : "Example World ETF" (fictitious, EUR, ticker EWE)
  Account  : "Example Cash Account" (EUR)
  Depot    : "Example Depot" (EUR)
  Period   : 2020-01-02 to 2024-12-31 (5 years = 60 months)
  Strategy : 20 quarterly deposits of €1,500 each, immediately invested.
             Buys whole shares; leftover cash stays in the account.

Expected values (to be confirmed by opening in Portfolio Performance):
  avg_monthly  = €30,000 / 60 months = €500.00  (exact by construction)
  TTWROR p.a.  ≈ 9–10% p.a.  (ETF price path: €100 → €158 over 5 years)

Workflow
--------
  1. Run this script:   python3 generate_example_portfolio.py
  2. Open example-portfolio.xml in Portfolio Performance (File → Open)
  3. Note the TTWROR p.a. shown in the Performance / Statement of Assets view.
  4. File → Save As → save as binary .portfolio
  5. Open the .portfolio in pp-viewer and compare TTWROR / Monthly values.

Usage
-----
  python3 generate_example_portfolio.py [--output example-portfolio.xml]
"""

import argparse
import textwrap
from pathlib import Path

# ---------------------------------------------------------------------------
# Transaction table
# (date, price_eur, shares_bought, buy_amount_cents)
# deposit is always €1,500 = 150000 cents
# buy_amount_cents = shares * price_eur * 100  (whole shares only)
# ---------------------------------------------------------------------------
DEPOSIT_CENTS = 150000  # €1,500 per quarter

TRANSACTIONS = [
    # date                price    shares  buy_cents
    #
    # IMPORTANT: deposits are placed one day AFTER the monthly price-change date
    # (which is always the 1st).  If they fell on the 1st, PP's daily rebalancing
    # would conflate the price jump with the deposit in a single delta computation,
    # artificially deflating the TTWROR.  January deposits naturally land on the
    # 2nd (New Year's Day is a market holiday); Q2/Q3/Q4 deposits are on the 2nd
    # for the same reason.  The few non-standard dates (Jan 3, Oct 3 etc.) in
    # 2022/2023 already satisfy this constraint.
    #
    # pp-viewer uses partition_point to find the latest price ≤ deposit date, so
    # a deposit on the 2nd correctly picks up the 1st-of-month price. ✓
    ("2020-01-02T12:00", 100.00,  15,     150000),   # Q1-2020
    ("2020-04-02T12:00",  87.00,  17,     147900),   # Q2-2020  was 04-01
    ("2020-07-02T12:00", 102.00,  14,     142800),   # Q3-2020  was 07-01
    ("2020-10-02T12:00",  98.00,  15,     147000),   # Q4-2020  was 10-01
    ("2021-01-02T12:00", 114.00,  13,     148200),   # Q1-2021
    ("2021-04-02T12:00", 124.00,  12,     148800),   # Q2-2021  was 04-01
    ("2021-07-02T12:00", 130.00,  11,     143000),   # Q3-2021  was 07-01
    ("2021-10-02T12:00", 136.00,  11,     149600),   # Q4-2021  was 10-01
    ("2022-01-03T12:00", 134.00,  11,     147400),   # Q1-2022  (Mon after NYD)
    ("2022-04-02T12:00", 120.00,  12,     144000),   # Q2-2022  was 04-01
    ("2022-07-02T12:00", 115.00,  13,     149500),   # Q3-2022  was 07-01
    ("2022-10-03T12:00", 108.00,  13,     140400),   # Q4-2022  (Mon, 01 = Sat)
    ("2023-01-02T12:00", 118.00,  12,     141600),   # Q1-2023
    ("2023-04-03T12:00", 122.00,  12,     146400),   # Q2-2023  (Mon, 01 = Sat)
    ("2023-07-03T12:00", 132.00,  11,     145200),   # Q3-2023  (Mon, 01 = Sat)
    ("2023-10-02T12:00", 122.00,  12,     146400),   # Q4-2023
    ("2024-01-02T12:00", 141.00,  10,     141000),   # Q1-2024
    ("2024-04-02T12:00", 145.00,  10,     145000),   # Q2-2024  was 04-01
    ("2024-07-02T12:00", 153.00,   9,     137700),   # Q3-2024  was 07-01
    ("2024-10-02T12:00", 152.00,   9,     136800),   # Q4-2024  was 10-01
]

# Monthly ETF price series.  Prices encoded as integers × 1e8 (PP convention).
# t = first day of each month.  price_at() in pp-viewer uses "latest price on
# or before the query date", so the 1st-of-month price covers the 2nd/3rd etc.
MONTHLY_PRICES = [
    # (date,        price_eur)
    ("2020-01-01",  100.00),
    ("2020-02-01",   98.00),
    ("2020-03-01",   78.00),
    ("2020-04-01",   87.00),
    ("2020-05-01",   93.00),
    ("2020-06-01",   99.00),
    ("2020-07-01",  102.00),
    ("2020-08-01",  107.00),
    ("2020-09-01",  104.00),
    ("2020-10-01",   98.00),
    ("2020-11-01",  108.00),
    ("2020-12-01",  112.00),
    ("2021-01-01",  114.00),
    ("2021-02-01",  116.00),
    ("2021-03-01",  120.00),
    ("2021-04-01",  124.00),
    ("2021-05-01",  126.00),
    ("2021-06-01",  128.00),
    ("2021-07-01",  130.00),
    ("2021-08-01",  133.00),
    ("2021-09-01",  131.00),
    ("2021-10-01",  136.00),
    ("2021-11-01",  134.00),
    ("2021-12-01",  138.00),
    ("2022-01-01",  134.00),
    ("2022-02-01",  128.00),
    ("2022-03-01",  125.00),
    ("2022-04-01",  120.00),
    ("2022-05-01",  118.00),
    ("2022-06-01",  110.00),
    ("2022-07-01",  115.00),
    ("2022-08-01",  113.00),
    ("2022-09-01",  106.00),
    ("2022-10-01",  108.00),
    ("2022-11-01",  114.00),
    ("2022-12-01",  112.00),
    ("2023-01-01",  118.00),
    ("2023-02-01",  116.00),
    ("2023-03-01",  120.00),
    ("2023-04-01",  122.00),
    ("2023-05-01",  124.00),
    ("2023-06-01",  128.00),
    ("2023-07-01",  132.00),
    ("2023-08-01",  130.00),
    ("2023-09-01",  127.00),
    ("2023-10-01",  122.00),
    ("2023-11-01",  130.00),
    ("2023-12-01",  138.00),
    ("2024-01-01",  141.00),
    ("2024-02-01",  145.00),
    ("2024-03-01",  148.00),
    ("2024-04-01",  145.00),
    ("2024-05-01",  148.00),
    ("2024-06-01",  150.00),
    ("2024-07-01",  153.00),
    ("2024-08-01",  155.00),
    ("2024-09-01",  154.00),
    ("2024-10-01",  152.00),
    ("2024-11-01",  156.00),
    ("2024-12-01",  158.00),
]

LATEST_DATE  = "2024-12-31"
LATEST_PRICE = 158.00

# Fixed UUIDs (simple, memorable, non-real)
UUID_SECURITY  = "aaaaaaaa-0001-0000-0000-000000000001"
UUID_ACCOUNT   = "bbbbbbbb-0001-0000-0000-000000000001"
UUID_PORTFOLIO = "cccccccc-0001-0000-0000-000000000001"
UPDATED_AT     = "2026-01-01T00:00:00.000000Z"

def uuid_dep(n: int) -> str:
    return f"dddddddd-{n:04d}-0000-0000-000000000001"

def uuid_buy_acc(n: int) -> str:
    return f"eeeeeeee-{n:04d}-0000-0000-000000000001"

def uuid_buy_port(n: int) -> str:
    return f"ffffffff-{n:04d}-0000-0000-000000000001"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def price_xml(eur: float) -> int:
    """Convert EUR price to PP integer representation (× 1e8)."""
    return round(eur * 1e8)

def shares_xml(n: int) -> int:
    """Convert whole-share count to PP integer representation (× 1e8)."""
    return n * 100_000_000


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------
def validate():
    errors = []
    for i, (date, price, shares, buy_cents) in enumerate(TRANSACTIONS, 1):
        expected = round(shares * price * 100)
        if expected != buy_cents:
            errors.append(f"  txn {i} ({date}): {shares} × {price} × 100 = {expected}, but buy_cents = {buy_cents}")
    if errors:
        print("VALIDATION ERRORS:")
        for e in errors:
            print(e)
        raise SystemExit(1)

    total_shares   = sum(t[2] for t in TRANSACTIONS)
    total_deposits = len(TRANSACTIONS) * DEPOSIT_CENTS / 100
    total_buy      = sum(t[3] for t in TRANSACTIONS) / 100
    cash_left      = total_deposits - total_buy
    final_nav      = total_shares * LATEST_PRICE + cash_left
    avg_monthly    = total_deposits / 60

    print(f"Validation OK")
    print(f"  Total shares    : {total_shares}")
    print(f"  Total deposits  : €{total_deposits:,.2f}  ({len(TRANSACTIONS)} × €{DEPOSIT_CENTS/100:.2f})")
    print(f"  Total invested  : €{total_buy:,.2f}")
    print(f"  Cash remainder  : €{cash_left:,.2f}")
    print(f"  Final NAV       : €{final_nav:,.2f}  (at €{LATEST_PRICE})")
    print(f"  Avg monthly     : €{avg_monthly:.2f}  (expect €500.00)")


# ---------------------------------------------------------------------------
# XML building blocks
# ---------------------------------------------------------------------------
def indent(text: str, spaces: int) -> str:
    pad = " " * spaces
    return "\n".join(pad + line if line else "" for line in text.splitlines())


def price_elements() -> str:
    lines = []
    for date, price in MONTHLY_PRICES:
        lines.append(f'        <price t="{date}" v="{price_xml(price)}"/>')
    return "\n".join(lines)


def security_block() -> str:
    return f"""\
  <securities>
    <security>
      <uuid>{UUID_SECURITY}</uuid>
      <name>Example World ETF</name>
      <currencyCode>EUR</currencyCode>
      <isin></isin>
      <tickerSymbol>EWE</tickerSymbol>
      <feed>MANUAL</feed>
      <prices>
{price_elements()}
      </prices>
      <latest t="{LATEST_DATE}" v="{price_xml(LATEST_PRICE)}">
        <high>-1</high>
        <low>-1</low>
        <volume>-1</volume>
      </latest>
      <attributes>
        <map/>
      </attributes>
      <events/>
      <isRetired>false</isRetired>
      <updatedAt>{UPDATED_AT}</updatedAt>
    </security>
  </securities>"""


def portfolio_transactions_block() -> str:
    """
    All 20 portfolio-transactions, embedded inside the first account-
    transaction's crossEntry > portfolio > transactions.

    portfolio-transaction[1] : crossEntry references the outer crossEntry
    portfolio-transaction[N] : crossEntry contains inline accountTransaction
    """
    lines = []

    for n, (date, price, shares, buy_cents) in enumerate(TRANSACTIONS, 1):
        if n == 1:
            # First portfolio transaction: crossEntry back-references the
            # outer account-transaction[2]'s crossEntry object.
            block = f"""\
                <portfolio-transaction>
                  <uuid>{uuid_buy_port(n)}</uuid>
                  <date>{date}</date>
                  <currencyCode>EUR</currencyCode>
                  <amount>{buy_cents}</amount>
                  <security reference="../../../../../../../../../securities/security"/>
                  <crossEntry class="buysell" reference="../../../.."/>
                  <shares>{shares_xml(shares)}</shares>
                  <updatedAt>{UPDATED_AT}</updatedAt>
                  <type>BUY</type>
                </portfolio-transaction>"""
        else:
            # Subsequent portfolio transactions: crossEntry contains the
            # account-side transaction defined inline (no separate entry in
            # the accounts list for this — the accounts list just has a
            # reference element pointing here).
            block = f"""\
                <portfolio-transaction>
                  <uuid>{uuid_buy_port(n)}</uuid>
                  <date>{date}</date>
                  <currencyCode>EUR</currencyCode>
                  <amount>{buy_cents}</amount>
                  <security reference="../../../../../../../../../securities/security"/>
                  <crossEntry class="buysell">
                    <portfolio reference="../../../.."/>
                    <portfolioTransaction reference="../.."/>
                    <account reference="../../../../../../../.."/>
                    <accountTransaction>
                      <uuid>{uuid_buy_acc(n)}</uuid>
                      <date>{date}</date>
                      <currencyCode>EUR</currencyCode>
                      <amount>{buy_cents}</amount>
                      <security reference="../../../../../../../../../../../securities/security"/>
                      <crossEntry class="buysell" reference="../.."/>
                      <shares>0</shares>
                      <updatedAt>{UPDATED_AT}</updatedAt>
                      <type>BUY</type>
                    </accountTransaction>
                  </crossEntry>
                  <shares>{shares_xml(shares)}</shares>
                  <updatedAt>{UPDATED_AT}</updatedAt>
                  <type>BUY</type>
                </portfolio-transaction>"""
        lines.append(block)

    return "\n".join(lines)


def account_transactions_block() -> str:
    """
    40 account-transaction entries:
      odd  positions (1, 3, 5, …): standalone DEPOSIT
      even positions (2, 4, 6, …): BUY
        - position 2 : full portfolio embedded
        - positions 4, 6, … : XPath reference into the embedded portfolio
    """
    lines = []
    n_txns = len(TRANSACTIONS)

    for n in range(1, n_txns + 1):
        date, price, shares, buy_cents = TRANSACTIONS[n - 1]

        # --- DEPOSIT ---
        dep_block = f"""\
        <account-transaction>
          <uuid>{uuid_dep(n)}</uuid>
          <date>{date}</date>
          <currencyCode>EUR</currencyCode>
          <amount>{DEPOSIT_CENTS}</amount>
          <shares>0</shares>
          <updatedAt>{UPDATED_AT}</updatedAt>
          <type>DEPOSIT</type>
        </account-transaction>"""
        lines.append(dep_block)

        # --- BUY ---
        if n == 1:
            # First BUY: embed the entire portfolio definition here.
            port_txns = portfolio_transactions_block()
            buy_block = f"""\
        <account-transaction>
          <uuid>{uuid_buy_acc(1)}</uuid>
          <date>{date}</date>
          <currencyCode>EUR</currencyCode>
          <amount>{buy_cents}</amount>
          <security reference="../../../../../securities/security"/>
          <crossEntry class="buysell">
            <portfolio>
              <uuid>{UUID_PORTFOLIO}</uuid>
              <name>Example Depot</name>
              <isRetired>false</isRetired>
              <referenceAccount reference="../../../../.."/>
              <transactions>
{port_txns}
              </transactions>
              <attributes>
                <map/>
              </attributes>
              <updatedAt>{UPDATED_AT}</updatedAt>
            </portfolio>
            <portfolioTransaction reference="../portfolio/transactions/portfolio-transaction"/>
            <account reference="../../../.."/>
            <accountTransaction reference="../.."/>
          </crossEntry>
          <shares>0</shares>
          <updatedAt>{UPDATED_AT}</updatedAt>
          <type>BUY</type>
        </account-transaction>"""
            lines.append(buy_block)
        else:
            # Subsequent BUYs: reference the inline accountTransaction that
            # lives inside portfolio-transaction[n]'s crossEntry.
            ref = (
                f"../account-transaction[2]/crossEntry/portfolio/transactions"
                f"/portfolio-transaction[{n}]/crossEntry/accountTransaction"
            )
            buy_ref = f'        <account-transaction reference="{ref}"/>'
            lines.append(buy_ref)

    return "\n".join(lines)


def build_xml() -> str:
    sec   = security_block()
    accts = account_transactions_block()

    return f"""\
<client>
  <version>60</version>
  <baseCurrency>EUR</baseCurrency>
{sec}
  <watchlists/>
  <accounts>
    <account>
      <uuid>{UUID_ACCOUNT}</uuid>
      <name>Example Cash Account</name>
      <currencyCode>EUR</currencyCode>
      <isRetired>false</isRetired>
      <transactions>
{accts}
      </transactions>
      <attributes>
        <map/>
      </attributes>
      <updatedAt>{UPDATED_AT}</updatedAt>
    </account>
  </accounts>
  <portfolios>
    <portfolio reference="../../accounts/account/transactions/account-transaction[2]/crossEntry/portfolio"/>
  </portfolios>
  <plans/>
  <taxonomies/>
  <dashboards/>
</client>
"""


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------
def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--output", default="example-portfolio.xml",
                        help="Output file path (default: example-portfolio.xml)")
    parser.add_argument("--validate-only", action="store_true",
                        help="Only run validation, do not write the file")
    args = parser.parse_args()

    validate()

    if args.validate_only:
        return

    out = Path(args.output)
    out.write_text(build_xml(), encoding="utf-8")
    print(f"Written: {out.resolve()}")


if __name__ == "__main__":
    main()
