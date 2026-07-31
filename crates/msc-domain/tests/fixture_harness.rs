//! Native counterpart to `tools/fixture-runner/run.py --selftest`: proves
//! the fixture-loading and comparison path works before any real domain
//! (P1.3 onward) is wired through it.

mod support;

#[test]
fn fixture_harness_selftest() {
    let pass = support::load(support::fixtures_dir().join("_selftest/pass.json"));
    assert!(
        support::full_compare(&pass),
        "pass.json: harness should report a match"
    );

    let fail = support::load(support::fixtures_dir().join("_selftest/fail.json"));
    assert!(
        !support::full_compare(&fail),
        "fail.json: harness should report a mismatch"
    );
}
