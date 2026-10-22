//! Money and duration formatting mirrored from `packages/core/src/format.ts`.
//! The renderer formats on every frame, so it cannot depend on `Intl` (ADR-0014).

/// `formatMoney`: symbol position follows the currency ($1,234.50 / 1.234,50 €),
/// separators follow the UI locale (es groups from five digits, like `Intl`).
pub fn format_money(value: f64, currency: &str, locale: &str) -> String {
    let english = is_english(locale);
    let sign = if value < 0.0 { "-" } else { "" };
    let amount = amount(abs_cents(value), english);
    let Some(symbol) = currency_symbol(currency) else {
        // `Intl` throws for unknown codes: the web falls back to `1.00 CODE`.
        return format!("{}{} {}", sign, plain_amount(abs_cents(value)), currency);
    };
    if currency == "EUR" {
        return format!("{}{} {}", sign, amount, symbol);
    }
    if symbol.ends_with(|c: char| c.is_ascii_alphabetic()) {
        format!("{}{} {}", sign, symbol, amount)
    } else {
        format!("{}{}{}", sign, symbol, amount)
    }
}

/// `formatMoneyCompact`: abbreviates with the short (en) or long (es, European) scale.
pub fn format_money_compact(value: f64, currency: &str, locale: &str) -> String {
    let suffixes: &[&str] = if is_english(locale) {
        &["", "K", "M", "B", "T"]
    } else {
        &["", "K", "M", " mil M", "B"]
    };
    let mut scaled = value;
    let mut unit = 0;
    while unit < suffixes.len() - 1 && (scaled.abs() * 100.0).round() / 100.0 >= 1000.0 {
        scaled /= 1000.0;
        unit += 1;
    }
    let text = format_money(scaled, currency, locale);
    let suffix = suffixes[unit];
    if suffix.is_empty() {
        return text;
    }
    match text.rfind(|c: char| c.is_ascii_digit()) {
        Some(index) => {
            let mut out = text;
            out.insert_str(index + 1, suffix);
            out
        }
        None => text,
    }
}

/// `formatDuration`: `HH:MM:SS`, negative values clamp to zero.
pub fn format_duration(ms: i64) -> String {
    let total = ms.max(0) / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

fn is_english(locale: &str) -> bool {
    locale
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .eq_ignore_ascii_case("en")
}

fn currency_symbol(currency: &str) -> Option<&'static str> {
    match currency {
        "EUR" => Some("€"),
        "USD" | "MXN" | "ARS" | "COP" | "CLP" => Some("$"),
        "GBP" => Some("£"),
        "BRL" => Some("R$"),
        "CHF" => Some("CHF"),
        _ => None,
    }
}

fn abs_cents(value: f64) -> u128 {
    (value.abs() * 100.0).round() as u128
}

/// `1234.5` -> `1,234.50` (en) / `1234,50` (es: groups from five digits, like `Intl`).
fn amount(cents: u128, english: bool) -> String {
    let (decimal, grouping) = if english { ('.', ',') } else { (',', '.') };
    let integer = (cents / 100).to_string();
    let min_len = if english { 4 } else { 5 };
    let mut out = group(&integer, grouping, min_len);
    out.push(decimal);
    out.push_str(&format!("{:02}", cents % 100));
    out
}

/// `toFixed(2)` style, no grouping: used by the unknown-currency fallback.
fn plain_amount(cents: u128) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

fn group(digits: &str, separator: char, min_len: usize) -> String {
    if digits.len() < min_len {
        return digits.to_string();
    }
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations_as_hhmmss() {
        assert_eq!(format_duration(0), "00:00:00");
        assert_eq!(format_duration(3_723_999), "01:02:03");
        assert_eq!(format_duration(-5), "00:00:00");
    }

    #[test]
    fn formats_money_with_two_decimals() {
        assert_eq!(format_money(1234.5, "USD", "en-US"), "$1,234.50");
    }

    #[test]
    fn puts_the_symbol_before_or_after_the_amount_by_currency() {
        assert_eq!(format_money(1234.5, "USD", "es"), "$1234,50");
        assert_eq!(format_money(1234.5, "EUR", "en"), "1,234.50 €");
        assert_eq!(format_money(1234.5, "EUR", "es"), "1234,50 €");
        assert_eq!(format_money(-3.0, "EUR", "en"), "-3.00 €");
        assert_eq!(format_money(1234567.89, "EUR", "es"), "1.234.567,89 €");
        assert_eq!(format_money(1234567.89, "USD", "en-US"), "$1,234,567.89");
    }

    #[test]
    fn falls_back_to_the_code_for_unknown_currencies() {
        assert_eq!(format_money(1.0, "NOPE!", "en"), "1.00 NOPE!");
    }

    #[test]
    fn abbreviates_with_the_scale_of_the_locale() {
        assert_eq!(format_money_compact(1_234_567.0, "EUR", "en"), "1.23M €");
        assert_eq!(format_money_compact(1_234_567.0, "EUR", "es"), "1,23M €");
        assert_eq!(format_money_compact(1_234.5, "EUR", "en"), "1.23K €");
        assert_eq!(format_money_compact(1_234.5, "USD", "es"), "$1,23K");
        assert_eq!(format_money_compact(1_500_000.0, "USD", "en-US"), "$1.50M");
        assert_eq!(
            format_money_compact(1_234_567_890_123.0, "EUR", "es"),
            "1,23B €"
        );
        assert_eq!(format_money_compact(999.0, "EUR", "es"), "999,00 €");
    }
}
