/*
Fee model and assumptions:
- eBay fee is charged on subtotal + shipping (if customer pays shipping) + sales tax.
- Sales tax is collected from the buyer and remitted by the seller, so tax is not profit.
- eBay fee has two independent parts: percentage fee (F) and fixed fee (D).
- If customer pays shipping, shipping is collected and passed through (no shipping cost in K).
- If seller pays shipping, shipping amount is folded into K as an extra out-of-pocket cost.
- We solve for sale price P algebraically, then round P up to the nearest cent so the
  final recomputed net profit meets or exceeds the target.
*/

use std::io::{self, Write};
use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};

#[derive(Clone, Copy)]
struct InputValue {
    value: Decimal,
    defaulted: bool,
}

struct Inputs {
    target_profit: InputValue,
    item_cost: InputValue,
    supplies_cost: InputValue,
    customer_pays_shipping: bool,
    customer_pays_shipping_defaulted: bool,
    shipping_amount: InputValue,
    tax_rate_percent: InputValue,
    standard_fee_rate_percent: InputValue,
    fixed_fee: InputValue,
}

#[derive(Clone, Copy)]
struct Calculation {
    subtotal_plus_ship: Decimal,
    tax: Decimal,
    fee_base: Decimal,
    standard_fee: Decimal,
    fixed_fee: Decimal,
    ebay_fee: Decimal,
    k: Decimal,
    total_costs: Decimal,
    net_profit: Decimal,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
    }
}

fn run() -> io::Result<()> {
    println!("eBay Target Profit Sale Price Calculator");
    println!("----------------------------------------");

    let default_target = Decimal::new(1000, 2);
    let default_zero = Decimal::ZERO;
    let default_standard_fee_rate_percent = Decimal::new(1325, 2);
    let default_fixed_fee = Decimal::new(40, 2);

    let target_profit = read_required_target_profit(default_target)?;
    let item_cost = read_optional_decimal(
        "2) Initial price paid for the item (cost basis) [default 0.00]: ",
        default_zero,
    )?;
    let supplies_cost = read_optional_decimal(
        "3) Shipping/supply costs paid out of pocket [default 0.00]: ",
        default_zero,
    )?;
    let (customer_pays_shipping, customer_pays_shipping_defaulted) = read_optional_yes_no(
        "4) Will the customer pay for shipping? (yes/no, default no): ",
        false,
    )?;
    let shipping_amount = read_optional_decimal(
        "5) Expected shipping amount (charged to customer or paid by seller) [default 0.00]: ",
        default_zero,
    )?;
    let tax_rate_percent = read_optional_decimal(
        "6) Sales tax rate (%) [default 0.00]: ",
        default_zero,
    )?;
    let standard_fee_rate_percent = read_optional_decimal(
        "7a) eBay standard final value fee rate (%) [default 13.25]: ",
        default_standard_fee_rate_percent,
    )?;
    let fixed_fee = read_optional_decimal(
        "7b) eBay fixed final value fee ($) [default 0.40]: ",
        default_fixed_fee,
    )?;

    let inputs = Inputs {
        target_profit,
        item_cost,
        supplies_cost,
        customer_pays_shipping,
        customer_pays_shipping_defaulted,
        shipping_amount,
        tax_rate_percent,
        standard_fee_rate_percent,
        fixed_fee,
    };

    let target_profit_amt = inputs.target_profit.value;
    let shipping_collected = if inputs.customer_pays_shipping {
        inputs.shipping_amount.value
    } else {
        Decimal::ZERO
    };

    let k = inputs.item_cost.value
        + inputs.supplies_cost.value
        + if inputs.customer_pays_shipping {
            Decimal::ZERO
        } else {
            inputs.shipping_amount.value
        };

    let t = percent_to_decimal(inputs.tax_rate_percent.value);
    let f = percent_to_decimal(inputs.standard_fee_rate_percent.value);
    let d = inputs.fixed_fee.value;

    // Algebra (single closed-form solve, no iteration):
    // Let M = (customer pays shipping ? S : 0)
    // Let D = fixed fee dollars
    // subtotal_plus_ship = P + M
    // tax                = (P + M) * T
    // fee_base           = (P + M) + tax
    // standard_fee       = fee_base * F
    // fixed_fee          = D
    // ebay_fee           = standard_fee + fixed_fee
    // net                = P + M - tax - ebay_fee - K
    //                    = (P + M) * (1 - T - F - T*F) - D - K
    // target_profit      = (P + M) * A - D - K, where A = (1 - T - F - T*F)
    // => P = (target_profit + D + K) / A - M
    let a = Decimal::ONE - t - f - (t * f);

    if a <= Decimal::ZERO {
        println!();
        println!("Unable to compute a valid sale price with the current rates.");
        println!(
            "The combined tax/fee factor makes the denominator <= 0 (A = {}).",
            a.round_dp(6)
        );
        println!("Try lower tax and/or fee rates.");
        print_input_summary(&inputs, k);
        return Ok(());
    }

    let raw_price = (target_profit_amt + d + k) / a - shipping_collected;
    let recommended_price = round_up_to_cent(non_negative(raw_price));
    let calc = recompute(recommended_price, &inputs);

    println!();
    println!("================ RECOMMENDED SALE PRICE ================");
    println!("                 ${}", fmt_money(recommended_price));
    println!("========================================================");

    print_input_summary(&inputs, calc.k);
    print_result_summary(recommended_price, calc, &inputs);

    Ok(())
}

fn read_required_target_profit(default_value: Decimal) -> io::Result<InputValue> {
    loop {
        let raw = prompt("1) Target profit (required, dollars; Enter for default 10.00): ")?;
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            println!("No target profit entered. Using default: ${}", fmt_money(default_value));
            return Ok(InputValue {
                value: default_value,
                defaulted: true,
            });
        }

        match parse_non_negative_decimal(trimmed) {
            Some(value) => {
                return Ok(InputValue {
                    value,
                    defaulted: false,
                })
            }
            None => {
                println!(
                    "Invalid required value. Please enter a non-negative number (for example: 10.00)."
                );
            }
        }
    }
}

fn read_optional_decimal(prompt_text: &str, default_value: Decimal) -> io::Result<InputValue> {
    let raw = prompt(prompt_text)?;
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(InputValue {
            value: default_value,
            defaulted: true,
        });
    }

    match parse_non_negative_decimal(trimmed) {
        Some(value) => Ok(InputValue {
            value,
            defaulted: false,
        }),
        None => {
            println!(
                "Warning: invalid optional value '{}'. Using default ${}.",
                trimmed,
                fmt_money(default_value)
            );
            Ok(InputValue {
                value: default_value,
                defaulted: true,
            })
        }
    }
}

fn read_optional_yes_no(prompt_text: &str, default_value: bool) -> io::Result<(bool, bool)> {
    let raw = prompt(prompt_text)?;
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok((default_value, true));
    }

    let normalized = trimmed.to_ascii_lowercase();
    let value = match normalized.as_str() {
        "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        _ => None,
    };

    match value {
        Some(v) => Ok((v, false)),
        None => {
            println!(
                "Warning: invalid yes/no value '{}'. Using default {}.",
                trimmed,
                if default_value { "yes" } else { "no" }
            );
            Ok((default_value, true))
        }
    }
}

fn prompt(label: &str) -> io::Result<String> {
    print!("{label}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn parse_non_negative_decimal(s: &str) -> Option<Decimal> {
    match Decimal::from_str(s) {
        Ok(value) if value >= Decimal::ZERO => Some(value),
        _ => None,
    }
}

fn percent_to_decimal(percent: Decimal) -> Decimal {
    percent / Decimal::new(100, 0)
}

fn recompute(price: Decimal, inputs: &Inputs) -> Calculation {
    let shipping_collected = if inputs.customer_pays_shipping {
        inputs.shipping_amount.value
    } else {
        Decimal::ZERO
    };
    let subtotal_plus_ship = price + shipping_collected;

    let t = percent_to_decimal(inputs.tax_rate_percent.value);
    let f = percent_to_decimal(inputs.standard_fee_rate_percent.value);

    let tax = subtotal_plus_ship * t;
    let fee_base = subtotal_plus_ship + tax;
    let standard_fee = fee_base * f;
    let fixed_fee = inputs.fixed_fee.value;
    let ebay_fee = standard_fee + fixed_fee;

    let k = inputs.item_cost.value
        + inputs.supplies_cost.value
        + if inputs.customer_pays_shipping {
            Decimal::ZERO
        } else {
            inputs.shipping_amount.value
        };

    let net_profit = price + shipping_collected - tax - ebay_fee - k;
    let total_costs = tax + ebay_fee + k;

    Calculation {
        subtotal_plus_ship,
        tax,
        fee_base,
        standard_fee,
        fixed_fee,
        ebay_fee,
        k,
        total_costs,
        net_profit,
    }
}

fn round_up_to_cent(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::ToPositiveInfinity)
}

fn non_negative(value: Decimal) -> Decimal {
    if value < Decimal::ZERO {
        Decimal::ZERO
    } else {
        value
    }
}

fn fmt_money(value: Decimal) -> String {
    let rounded = value.round_dp(2);
    format!("{rounded:.2}")
}

fn fmt_percent(value: Decimal) -> String {
    let rounded = value.round_dp(4);
    format!("{rounded:.4}")
}

fn line(label: &str, value: &str, defaulted: bool) {
    let source = if defaulted { "default" } else { "user" };
    println!("{label:<52} {value:>14}   ({source})");
}

fn print_input_summary(inputs: &Inputs, k: Decimal) {
    println!();
    println!("Input Summary");
    println!("-------------");
    line(
        "Target profit:",
        &format!("${}", fmt_money(inputs.target_profit.value)),
        inputs.target_profit.defaulted,
    );
    line(
        "Initial item cost:",
        &format!("${}", fmt_money(inputs.item_cost.value)),
        inputs.item_cost.defaulted,
    );
    line(
        "Shipping/supply out-of-pocket costs:",
        &format!("${}", fmt_money(inputs.supplies_cost.value)),
        inputs.supplies_cost.defaulted,
    );
    line(
        "Customer pays shipping:",
        if inputs.customer_pays_shipping { "yes" } else { "no" },
        inputs.customer_pays_shipping_defaulted,
    );
    line(
        "Shipping amount (S):",
        &format!("${}", fmt_money(inputs.shipping_amount.value)),
        inputs.shipping_amount.defaulted,
    );
    line(
        "Sales tax rate:",
        &format!("{}%", fmt_percent(inputs.tax_rate_percent.value)),
        inputs.tax_rate_percent.defaulted,
    );
    line(
        "eBay standard final value fee rate:",
        &format!("{}%", fmt_percent(inputs.standard_fee_rate_percent.value)),
        inputs.standard_fee_rate_percent.defaulted,
    );
    line(
        "eBay fixed final value fee:",
        &format!("${}", fmt_money(inputs.fixed_fee.value)),
        inputs.fixed_fee.defaulted,
    );
    println!("{:<52} {:>14}", "Computed seller out-of-pocket costs (K):", format!("${}", fmt_money(k)));
}

fn print_result_summary(price: Decimal, calc: Calculation, inputs: &Inputs) {
    println!();
    println!("Computation At Recommended Price");
    println!("--------------------------------");
    println!("{:<52} {:>14}", "Recommended sale price (P):", format!("${}", fmt_money(price)));
    println!(
        "{:<52} {:>14}",
        "Subtotal + shipping used for tax:",
        format!("${}", fmt_money(calc.subtotal_plus_ship))
    );
    println!("{:<52} {:>14}", "Sales tax:", format!("${}", fmt_money(calc.tax)));
    println!("{:<52} {:>14}", "Fee base:", format!("${}", fmt_money(calc.fee_base)));
    let standard_fee_label = format!(
        "Standard final value fee ({}% of ${}):",
        fmt_percent(inputs.standard_fee_rate_percent.value),
        fmt_money(calc.fee_base)
    );
    println!(
        "{:<52} {:>14}",
        standard_fee_label,
        format!("${}", fmt_money(calc.standard_fee))
    );
    println!(
        "{:<52} {:>14}",
        "Fixed final value fee:",
        format!("${}", fmt_money(calc.fixed_fee))
    );
    println!("{:<52} {:>14}", "eBay fee total:", format!("${}", fmt_money(calc.ebay_fee)));
    println!("{:<52} {:>14}", "Total costs (tax + fee + K):", format!("${}", fmt_money(calc.total_costs)));
    println!("{:<52} {:>14}", "Final net profit:", format!("${}", fmt_money(calc.net_profit)));
}
