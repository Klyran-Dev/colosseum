use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tax_engine::{
    BaseReference, PriceMode, Rate, RateSpec, RoundingMode, RoundingSpec, RuleSet, TaxBase,
    TaxComponentRule, TaxFacts,
};

fn benchmark_rule() -> RuleSet {
    RuleSet {
        id: "benchmark-sales-tax".to_string(),
        version: 1,
        components: vec![
            TaxComponentRule {
                id: "state".to_string(),
                label: "State tax".to_string(),
                base: TaxBase::single(BaseReference::Net),
                rate: RateSpec::Flat(Rate::new(7, 100).expect("benchmark rate is valid")),
                price_mode: PriceMode::Exclusive,
                credit_atomic: 0,
                floor_atomic: None,
                cap_atomic: None,
                rounding: RoundingSpec {
                    mode: RoundingMode::HalfEven,
                },
                threshold: None,
                exemption_codes: Vec::new(),
            },
            TaxComponentRule {
                id: "local".to_string(),
                label: "Local tax".to_string(),
                base: TaxBase {
                    sources: vec![
                        BaseReference::Net,
                        BaseReference::Component("state".to_string()),
                    ],
                },
                rate: RateSpec::Flat(Rate::new(2, 100).expect("benchmark rate is valid")),
                price_mode: PriceMode::Exclusive,
                credit_atomic: 0,
                floor_atomic: Some(1),
                cap_atomic: Some(10_000),
                rounding: RoundingSpec {
                    mode: RoundingMode::HalfEven,
                },
                threshold: None,
                exemption_codes: Vec::new(),
            },
        ],
    }
}

fn benchmark_facts() -> TaxFacts {
    TaxFacts {
        gross_atomic: 10_900,
        net_atomic: 10_000,
        currency: "USD".to_string(),
        jurisdiction: "US-CA".to_string(),
        jurisdiction_resolution: None,
        exemption_codes: Vec::new(),
        cumulative_period: None,
    }
}

fn tax_engine_benchmarks(c: &mut Criterion) {
    let rule = benchmark_rule();
    let facts = benchmark_facts();
    let compiled = rule.compile().expect("benchmark rule compiles");

    c.bench_function("tax_engine_compile", |b| {
        b.iter(|| black_box(rule.compile().expect("benchmark rule compiles")))
    });
    c.bench_function("tax_engine_uncached_calculate", |b| {
        b.iter(|| black_box(tax_engine::calculate(black_box(&rule), black_box(&facts))))
    });
    c.bench_function("tax_engine_compiled_calculate", |b| {
        b.iter(|| black_box(compiled.calculate(black_box(&facts))))
    });
}

criterion_group!(benches, tax_engine_benchmarks);
criterion_main!(benches);
