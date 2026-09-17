# eBay Calculator

A single-file Rust terminal app that calculates the sale price you should list on eBay so your net profit meets or exceeds a target after tax, fees, and costs.

The calculator uses:
- Rust standard library
- rust_decimal for currency-safe decimal math

## What this solves

Given your target profit and expected costs, the app solves for the required sale price using a closed-form algebraic equation (no iterative guessing), then rounds up to the nearest cent so you do not undershoot your target.

## Fee model and assumptions

- Sales tax is charged to the buyer and remitted by the seller.
- Sales tax is not part of seller profit.
- eBay final value fee is applied to subtotal + shipping (if charged) + tax.
- A fixed eBay fee of $0.40 is always added.
- If the customer pays shipping, that shipping amount is treated as pass-through and is not added to seller out-of-pocket shipping cost.
- If the seller pays shipping, shipping is added to seller out-of-pocket costs.

### Variables

- P: sale price to solve for
- R: target profit
- T: sales tax rate as decimal (example: 0.0725)
- F: standard final value fee rate as decimal (example: 0.1325)
- D: fixed final value fee dollars (default 0.40)
- S: shipping amount from prompt
- M: collected shipping, where M = S if customer pays shipping, else 0
- K: seller out-of-pocket costs

### Formula used

1. subtotal_plus_ship = P + M
2. tax = (P + M) * T
3. fee_base = subtotal_plus_ship + tax
4. standard_fee = fee_base * F
5. fixed_fee = D
6. ebay_fee = standard_fee + fixed_fee
7. net = P + M - tax - ebay_fee - K

After simplification:

- net = (P + M) * (1 - T - F - T*F) - D - K

Set net = target R and solve for P:

- P = (R + D + K) / (1 - T - F - T*F) - M

If the denominator is less than or equal to zero, a valid sale price cannot be computed for those rates.

## Interactive inputs

The app prompts in this order:

1. Target profit (required, but Enter uses default 10.00 and the app tells you).
2. Initial item cost.
3. Out-of-pocket shipping/supply costs.
4. Whether customer pays shipping (yes/no, default no).
5. Shipping amount (default 0.00).
6. Sales tax rate percent (default 0.00).
7. eBay fee inputs:
	- 7a. Standard final value fee rate percent (default 13.25).
	- 7b. Fixed final value fee dollars (default 0.40).

Validation behavior:

- Required invalid input: reprompt.
- Optional invalid input: warning + fallback to default.
- Empty optional input: default.
- Negative values are treated as invalid.
- Yes/no matching is case-insensitive and trims whitespace.

## Output summary

The app prints:

- Recommended sale price (emphasized)
- Input summary with whether each field was user-entered or defaulted
- Recomputed tax, eBay fee, total costs, and final net profit at the rounded sale price
- Separate fee lines:
	- standard fee
	- fixed fee
	- eBay fee total

This verifies the rounded recommendation still meets or exceeds the target.

## Project layout

- Cargo.toml
- src/main.rs

## Build and run

### Prerequisite

Install Rust from rustup:

- https://rustup.rs

If Cargo reports no default toolchain, set one:

```bash
rustup default stable
```

### Linux (bash)

```bash
cd /path/to/ebay-calculator
cargo build --release
./target/release/ebay-calculator
```

Quick run without separate build:

```bash
cd /path/to/ebay-calculator
cargo run
```

### Windows (PowerShell)

```powershell
cd C:\path\to\ebay-calculator
cargo build --release
.\target\release\ebay-calculator.exe
```

Quick run without separate build:

```powershell
cd C:\path\to\ebay-calculator
cargo run
```

### Windows (Command Prompt)

```bat
cd C:\path\to\ebay-calculator
cargo build --release
target\release\ebay-calculator.exe
```

## Example usage flow

1. Enter target profit (or press Enter to use 10.00).
2. Fill only the fields you know.
3. Press Enter on optional unknowns.
4. Read the recommended sale price and verification summary.

## Notes

- All money math is done with Decimal, not floating point.
- The app is intentionally single-file and avoids OS-specific code.