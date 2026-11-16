//! 台账金额、归属、完整性与 Excel 的行为回归

use enterprise_ledger::{
    data::*,
    export,
    report::{self, Report},
};
use gateway_plugin_sdk::call::services::usage::*;
use rust_decimal::Decimal;
use std::io::{Cursor, Read};

fn settings() -> Settings {
    Settings {
        pools: vec![Pool {
            id: "pool".into(),
            name: "共享池".into(),
            basis: Basis::Tokens,
            account_ids: vec![],
            key_ids: vec![],
            provider: None,
            shares: vec![],
        }],
        assignments: vec![Assignment {
            id: "map-a".into(),
            key_id: "key-a".into(),
            person: "张三".into(),
            department: "研发部".into(),
            project: "项目A".into(),
            start_date: "2026-10-01".into(),
            end_date: None,
        }],
        payments: vec![Payment {
            id: "pay".into(),
            paid_date: "2026-10-01".into(),
            supplier: "供应商".into(),
            voucher: "invoice-1".into(),
            kind: PaymentKind::Subscription,
            pool_id: "pool".into(),
            currency: "CNY".into(),
            paid_amount: "100.00".into(),
            allocation_base: "100.00".into(),
            start_date: "2026-10-01".into(),
            end_date: "2026-11-01".into(),
            original_payment_id: None,
            notes: String::new(),
        }],
        ..Settings::default()
    }
}
fn query() -> Query {
    Query {
        start_date: "2026-10-01".into(),
        end_date: "2026-10-31".into(),
    }
}
fn fact(id: usize, tokens: Option<u64>, cost: Option<&str>) -> UsageRecordFact {
    UsageRecordFact {
        id: format!("request-{id:05}"),
        data: UsageRecordData {
            client_key_ref: if id.is_multiple_of(2) {
                "key-a"
            } else {
                "key-b"
            }
            .into(),
            provider: Some("openai".into()),
            account_ref: Some("account-1".into()),
            requested_model: Some("alias".into()),
            model: Some("model-a".into()),
            protocol: "openai".into(),
            operation: "responses".into(),
            endpoint: "/v1/responses".into(),
            client_transport: "http".into(),
            service_tier: None,
            started_at_ms: query().period("Asia/Shanghai").unwrap().start_ms + 3_600_000,
            completed_at_ms: Some(query().period("Asia/Shanghai").unwrap().start_ms + 3_600_001),
            outcome: UsageOutcome::Succeeded,
            tokens: UsageTokens {
                total_tokens: tokens.map(UsageCount),
                ..UsageTokens::default()
            },
            cost: cost.map_or(UsageCost::Unavailable, |v| UsageCost::Known {
                source: UsageCostSource::Calculated,
                amount: v.into(),
                currency: "USD".into(),
            }),
        },
        user_agent: None,
        reasoning_effort: None,
        subagent_kind: None,
        request_kind: None,
        routing_scope: None,
        routing_group_refs: vec![],
        recovered: false,
        request_counted: true,
        usage_counted: true,
    }
}
fn build(s: &Settings, f: Vec<UsageRecordFact>) -> Report {
    report::build(s, query(), f, 123).unwrap()
}
fn sum(values: impl Iterator<Item = String>) -> Decimal {
    values.map(|s| decimal(&s).unwrap()).sum()
}

#[test]
fn totals_and_all_group_views_reconcile() {
    let r = build(
        &settings(),
        vec![fact(0, Some(1), Some("0.1")), fact(1, Some(2), Some("0.2"))],
    );
    assert_eq!(r.metrics.requests, 2);
    assert_eq!(r.metrics.total_tokens, "3");
    assert_eq!(
        sum(r.allocations.iter().map(|a| a.amount.clone())),
        Decimal::from(100)
    );
    for groups in [&r.people, &r.departments, &r.projects] {
        assert_eq!(
            sum(groups
                .iter()
                .flat_map(|g| g.allocations.iter().map(|m| m.amount.clone()))),
            Decimal::from(100)
        );
    }
}

#[test]
fn unknown_cost_blocks_entire_amount_pool() {
    let mut s = settings();
    s.pools[0].basis = Basis::Cost;
    let r = build(
        &s,
        vec![fact(0, Some(1), Some("1")), fact(1, Some(1), None)],
    );
    assert!(r.allocations.is_empty());
    assert_eq!(
        decimal(&r.unallocated[0].amount).unwrap(),
        Decimal::from(100)
    );
    assert_eq!(r.metrics.unknown_costs, 1);
}

#[test]
fn known_zero_cost_is_not_unknown() {
    let mut s = settings();
    s.pools[0].basis = Basis::Cost;
    let r = build(&s, vec![fact(0, Some(1), Some("0"))]);
    assert_eq!(r.metrics.unknown_costs, 0);
    assert!(r.allocations.is_empty());
    assert!(r.expenses[0].reason.as_ref().unwrap().contains("零"));
}

#[test]
fn missing_token_blocks_token_pool() {
    let r = build(&settings(), vec![fact(0, None, Some("1"))]);
    assert_eq!(r.metrics.unknown_tokens, 1);
    assert!(r.allocations.is_empty());
}

#[test]
fn tiny_payments_never_create_negative_allocations() {
    let mut s = settings();
    s.payments[0].paid_amount = "0.03".into();
    s.payments[0].allocation_base = "0.03".into();
    let r = build(&s, (0..6).map(|i| fact(i, Some(1), Some("1"))).collect());
    assert!(
        r.allocations
            .iter()
            .all(|a| decimal(&a.amount).unwrap() >= Decimal::ZERO)
    );
    assert_eq!(
        sum(r.allocations.iter().map(|a| a.amount.clone())),
        decimal("0.03").unwrap()
    );
    assert_eq!(
        r.allocations
            .iter()
            .filter(|a| decimal(&a.amount).unwrap() > Decimal::ZERO)
            .count(),
        3
    );
}

#[test]
fn refund_and_recharge_keep_cash_and_allocation_separate() {
    let mut s = settings();
    let mut refund = s.payments[0].clone();
    refund.id = "refund".into();
    refund.voucher = "invoice-2".into();
    refund.kind = PaymentKind::Refund;
    refund.paid_amount = "-20".into();
    refund.allocation_base = "-20".into();
    refund.original_payment_id = Some("pay".into());
    s.payments.push(refund);
    let mut charge = s.payments[0].clone();
    charge.id = "top-up".into();
    charge.voucher = "invoice-3".into();
    charge.kind = PaymentKind::Recharge;
    charge.paid_amount = "500".into();
    charge.allocation_base = "0".into();
    s.payments.push(charge);
    let r = build(&s, vec![fact(0, Some(10), Some("5"))]);
    assert_eq!(decimal(&r.cash[0].amount).unwrap(), Decimal::from(580));
    assert_eq!(decimal(&r.allocated[0].amount).unwrap(), Decimal::from(80));
}

#[test]
fn mixed_weight_currencies_are_not_added() {
    let mut s = settings();
    s.pools[0].basis = Basis::Cost;
    let mut other = fact(1, Some(1), Some("1"));
    other.data.cost = UsageCost::Known {
        source: UsageCostSource::Calculated,
        amount: "1".into(),
        currency: "CNY".into(),
    };
    let r = build(&s, vec![fact(0, Some(1), Some("1")), other]);
    assert!(r.allocations.is_empty());
    assert_eq!(r.metrics.usage_costs.len(), 2);
}

#[test]
fn attribution_periods_cannot_overlap() {
    let mut s = settings();
    let mut a = s.assignments[0].clone();
    a.id = "map-b".into();
    a.start_date = "2026-10-15".into();
    s.assignments.push(a);
    assert!(s.validate().unwrap_err().contains("重叠"));
    s.assignments[0].end_date = Some("2026-10-15".into());
    assert!(s.validate().is_ok());
}

#[test]
fn duplicate_vouchers_and_recharge_expense_are_rejected() {
    let mut s = settings();
    let mut p = s.payments[0].clone();
    p.id = "other".into();
    s.payments.push(p);
    assert!(s.validate().is_err());
    s.payments.pop();
    s.payments[0].kind = PaymentKind::Recharge;
    assert!(s.validate().is_err());
}

#[test]
fn excluded_requests_do_not_pollute_totals_or_weights() {
    let mut excluded = fact(1, Some(1_000_000), Some("1000"));
    excluded.request_counted = false;
    excluded.usage_counted = false;
    let r = build(&settings(), vec![fact(0, Some(1), Some("0")), excluded]);
    assert_eq!(r.metrics.requests, 1);
    assert_eq!(r.metrics.total_tokens, "1");
    assert_eq!(r.allocations.len(), 1);
}

#[test]
fn snapshot_scan_rejects_duplicates_and_out_of_range() {
    let a = fact(0, Some(1), Some("1"));
    assert!(report::build(&settings(), query(), vec![a.clone(), a], 1).is_err());
    let mut a = fact(0, Some(1), Some("1"));
    a.data.started_at_ms -= 10_000_000;
    assert!(report::build(&settings(), query(), vec![a], 1).is_err());
}

#[test]
fn xlsx_contains_all_pages_and_stores_formula_like_names_as_text() {
    let mut s = settings();
    s.assignments[0].person = "=HYPERLINK(\"https://example.invalid\")".into();
    let r = build(
        &s,
        (0..1500)
            .map(|i| fact(i, Some(10), Some("0.001")))
            .collect(),
    );
    let bytes = export::workbook(&r, &s).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut strings = String::new();
    zip.by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut strings)
        .unwrap();
    assert!(strings.contains("request-01499"));
    assert!(strings.contains("=HYPERLINK"));
    let mut detail = String::new();
    zip.by_name("xl/worksheets/sheet5.xml")
        .unwrap()
        .read_to_string(&mut detail)
        .unwrap();
    assert!(detail.contains("r=\"1506\""));
    assert!(!detail.contains("<f>"));
    assert_eq!(
        zip.file_names()
            .filter(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
            .count(),
        7
    );
}

#[test]
fn budget_remaining_and_zero_denominator_are_explicit() {
    let mut s = settings();
    s.budgets.push(Budget {
        id: "budget-a".into(),
        scope: BudgetScope::Department,
        name: "研发部".into(),
        currency: "CNY".into(),
        amount: "80".into(),
        start_date: "2026-10-01".into(),
        end_date: "2026-11-01".into(),
    });
    let r = build(&s, vec![fact(0, Some(1), Some("1"))]);
    let status = &r
        .departments
        .iter()
        .find(|g| g.name == "研发部")
        .unwrap()
        .budget_status[0];
    assert_eq!(decimal(&status.remaining).unwrap(), Decimal::from(-20));
    assert_eq!(
        decimal(status.usage_percent.as_ref().unwrap()).unwrap(),
        decimal("125").unwrap()
    );
    s.budgets[0].amount = "0".into();
    let r = build(&s, vec![fact(0, Some(1), Some("1"))]);
    assert!(
        r.departments
            .iter()
            .find(|g| g.name == "研发部")
            .unwrap()
            .budget_status[0]
            .usage_percent
            .is_none()
    );
}
