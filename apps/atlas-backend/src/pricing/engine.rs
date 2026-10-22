/*!
 * Pure cart arithmetic: no I/O, no database. This file is the single
 * definition of Atlas's rounding rules, so the quote endpoint, live
 * checkout, offline sync and returns can never disagree about a total.
 *
 * The rules, in order, per line:
 *   1. gross = round(unit price x quantity)
 *   2. line discount (an amount, or a share of gross), clamped to [0, gross]
 *   3. the cart-level discount is split across lines in proportion to what
 *      is left after line discounts, by largest remainder, so the shares
 *      sum to the cart discount exactly
 *   4. net = gross - line discount - allocated cart discount
 *   5. tax on net: exclusive adds round(net x rate) on top; inclusive
 *      carves net - round(net / (1 + rate)) out of it
 *
 * Header totals are sums of line values, never recomputed from scratch, so
 * a sale's header always reconciles with its lines.
 */
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

pub const BASIS_POINTS: i64 = 10_000;

/// The only rounding call in the codebase: half away from zero, to whole minor units (kobo).
pub fn round_minor(value: Decimal) -> i64 {
    return value
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .to_i64()
        .expect("amount out of i64 range");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discount {
    Amount(i64),
    BasisPoints(i64),
}

impl Discount {
    /// The discount in minor units against `base`, clamped so it can never be negative or exceed what it discounts.
    pub fn resolve(self, base: i64) -> i64 {
        let raw = match self {
            Discount::Amount(amount) => amount,
            Discount::BasisPoints(bp) => round_minor(Decimal::from(base) * Decimal::from(bp) / Decimal::from(BASIS_POINTS)),
        };
        return raw.clamp(0, base.max(0));
    }
}

#[derive(Debug, Clone)]
pub struct LineInput {
    pub unit_price_minor: i64,
    pub quantity: Decimal,
    pub discount: Option<Discount>,
    pub tax_rate_basis_points: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineTotals {
    pub gross_minor: i64,
    pub line_discount_minor: i64,
    pub allocated_cart_discount_minor: i64,
    pub net_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartTotals {
    pub lines: Vec<LineTotals>,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub cart_discount_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
}

pub fn price_cart(lines: &[LineInput], cart_discount: Option<Discount>, prices_include_tax: bool) -> CartTotals {
    let grosses: Vec<i64> = lines
        .iter()
        .map(|line| round_minor(Decimal::from(line.unit_price_minor) * line.quantity))
        .collect();
    let line_discounts: Vec<i64> = lines
        .iter()
        .zip(&grosses)
        .map(|(line, gross)| line.discount.map(|d| d.resolve(*gross)).unwrap_or(0))
        .collect();
    let after_line: Vec<i64> = grosses.iter().zip(&line_discounts).map(|(g, d)| g - d).collect();
    let cart_base: i64 = after_line.iter().sum();
    let cart_discount_minor = cart_discount.map(|d| d.resolve(cart_base)).unwrap_or(0);
    let allocated = allocate(cart_discount_minor, &after_line);

    let mut totals = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let net = after_line[i] - allocated[i];
        let (tax, total) = tax_on(net, line.tax_rate_basis_points, prices_include_tax);
        totals.push(LineTotals {
            gross_minor: grosses[i],
            line_discount_minor: line_discounts[i],
            allocated_cart_discount_minor: allocated[i],
            net_minor: net,
            tax_minor: tax,
            total_minor: total,
        });
    }

    return CartTotals {
        subtotal_minor: totals.iter().map(|l| l.gross_minor).sum(),
        discount_minor: totals.iter().map(|l| l.line_discount_minor + l.allocated_cart_discount_minor).sum(),
        cart_discount_minor,
        tax_minor: totals.iter().map(|l| l.tax_minor).sum(),
        total_minor: totals.iter().map(|l| l.total_minor).sum(),
        lines: totals,
    };
}

/// (tax, amount payable) for a discounted line amount.
pub fn tax_on(net: i64, rate_basis_points: i64, prices_include_tax: bool) -> (i64, i64) {
    if rate_basis_points == 0 || net == 0 {
        return (0, net);
    }
    if prices_include_tax {
        let ex_tax = round_minor(Decimal::from(net) * Decimal::from(BASIS_POINTS) / Decimal::from(BASIS_POINTS + rate_basis_points));
        return (net - ex_tax, net);
    }
    let tax = round_minor(Decimal::from(net) * Decimal::from(rate_basis_points) / Decimal::from(BASIS_POINTS));
    return (tax, net + tax);
}

/**
 * Splits `amount` across `weights` in proportion, by the largest-remainder method: every share is floored, then the leftover minor units go one each to the largest fractional remainders (earliest index wins a tie).
 * The shares always sum to exactly `amount`, which a plain per-line round() does not guarantee.
 */
pub fn allocate(amount: i64, weights: &[i64]) -> Vec<i64> {
    let total: i64 = weights.iter().sum();
    if amount == 0 || total <= 0 {
        return vec![0; weights.len()];
    }
    let mut shares = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    for (i, weight) in weights.iter().enumerate() {
        let exact = i128::from(amount) * i128::from(*weight);
        shares.push((exact / i128::from(total)) as i64);
        remainders.push((exact % i128::from(total), i));
    }
    let mut leftover = amount - shares.iter().sum::<i64>();
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in remainders {
        if leftover == 0 {
            break;
        }
        shares[i] += 1;
        leftover -= 1;
    }
    return shares;
}

/// What cash rounding adds to (or takes off) `total`, rounded half away from zero to the nearest `increment`.
pub fn cash_rounding_adjustment(total: i64, increment: i64) -> i64 {
    if increment <= 1 {
        return 0;
    }
    let rounded = round_minor(Decimal::from(total) / Decimal::from(increment)) * increment;
    return rounded - total;
}

/**
 * The refund owed for returning `quantity` of a line that sold `sold_quantity` for `line_total`.
 * Pro-rata on what the customer actually paid; when the return brings the line to fully returned, it's whatever is left of `line_total` after earlier refunds, so repeated partial returns can never leak or overpay a kobo through rounding.
 */
#[allow(dead_code)]
pub fn prorated_refund(line_total: i64, sold_quantity: Decimal, already_returned: Decimal, already_refunded: i64, quantity: Decimal) -> i64 {
    if already_returned + quantity >= sold_quantity {
        return line_total - already_refunded;
    }
    return round_minor(Decimal::from(line_total) * quantity / sold_quantity);
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn line(price: i64, quantity: &str, tax_bp: i64) -> LineInput {
        return LineInput {
            unit_price_minor: price,
            quantity: quantity.parse().unwrap(),
            discount: None,
            tax_rate_basis_points: tax_bp,
        };
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(round_minor("2.5".parse().unwrap()), 3);
        assert_eq!(round_minor("2.4999".parse().unwrap()), 2);
        assert_eq!(round_minor("-2.5".parse().unwrap()), -3);
    }

    #[test]
    fn inclusive_tax_is_carved_out_of_the_shelf_price() {
        // N1,075.00 at 7.5% inclusive: N1,000.00 net of tax, N75.00 tax.
        let totals = price_cart(&[line(107_500, "1", 750)], None, true);
        assert_eq!(totals.tax_minor, 7_500);
        assert_eq!(totals.total_minor, 107_500);
    }

    #[test]
    fn exclusive_tax_is_added_on_top() {
        let totals = price_cart(&[line(100_000, "2", 750)], None, false);
        assert_eq!(totals.subtotal_minor, 200_000);
        assert_eq!(totals.tax_minor, 15_000);
        assert_eq!(totals.total_minor, 215_000);
    }

    #[test]
    fn fractional_quantities_bill_the_fraction() {
        let totals = price_cart(&[line(1_001, "0.5", 0)], None, true);
        // 500.5 rounds half away from zero to 501.
        assert_eq!(totals.total_minor, 501);
    }

    #[test]
    fn zero_rate_has_no_tax() {
        let totals = price_cart(&[line(999, "3", 0)], None, false);
        assert_eq!(totals.tax_minor, 0);
        assert_eq!(totals.total_minor, 2_997);
    }

    #[test]
    fn an_awkward_cart_discount_allocates_exactly() {
        let lines = [line(10_000, "1", 0), line(10_000, "1", 0), line(10_000, "1", 0)];
        let totals = price_cart(&lines, Some(Discount::Amount(10_001)), true);
        let allocated: Vec<i64> = totals.lines.iter().map(|l| l.allocated_cart_discount_minor).collect();
        assert_eq!(allocated.iter().sum::<i64>(), 10_001);
        assert_eq!(allocated, vec![3_334, 3_334, 3_333]);
        assert_eq!(totals.total_minor, 30_000 - 10_001);
        assert_eq!(totals.discount_minor, 10_001);
    }

    #[test]
    fn discounts_never_exceed_what_they_discount() {
        let mut discounted = line(500, "1", 0);
        discounted.discount = Some(Discount::Amount(900));
        let totals = price_cart(&[discounted], Some(Discount::Amount(50)), true);
        assert_eq!(totals.lines[0].line_discount_minor, 500);
        assert_eq!(totals.cart_discount_minor, 0);
        assert_eq!(totals.total_minor, 0);
    }

    #[test]
    fn percentage_discounts_resolve_against_gross() {
        let mut discounted = line(2_000, "3", 0);
        discounted.discount = Some(Discount::BasisPoints(1_000));
        let totals = price_cart(&[discounted], None, true);
        assert_eq!(totals.lines[0].line_discount_minor, 600);
        assert_eq!(totals.total_minor, 5_400);
    }

    #[test]
    fn header_totals_are_sums_of_lines() {
        let lines = [line(33_333, "1", 750), line(12_345, "2", 750), line(7, "3", 0)];
        let totals = price_cart(&lines, Some(Discount::BasisPoints(333)), false);
        assert_eq!(totals.total_minor, totals.lines.iter().map(|l| l.total_minor).sum::<i64>());
        assert_eq!(totals.tax_minor, totals.lines.iter().map(|l| l.tax_minor).sum::<i64>());
        assert_eq!(
            totals.total_minor,
            totals.subtotal_minor - totals.discount_minor + totals.tax_minor
        );
    }

    #[test]
    fn cash_rounding_goes_to_the_nearest_increment() {
        assert_eq!(cash_rounding_adjustment(10_240, 500), -240);
        assert_eq!(cash_rounding_adjustment(10_250, 500), 250);
        assert_eq!(cash_rounding_adjustment(10_260, 500), 240);
        assert_eq!(cash_rounding_adjustment(10_260, 1), 0);
    }

    #[test]
    fn repeated_partial_refunds_sum_to_the_line_total() {
        let sold: Decimal = "3".parse().unwrap();
        let one: Decimal = "1".parse().unwrap();
        let line_total = 299_999;
        let first = prorated_refund(line_total, sold, Decimal::ZERO, 0, one);
        let second = prorated_refund(line_total, sold, one, first, one);
        let third = prorated_refund(line_total, sold, one + one, first + second, one);
        assert_eq!(first + second + third, line_total);
    }
}
