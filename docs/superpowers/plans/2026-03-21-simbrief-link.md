# SimBrief Dispatch Link Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a SimBrief dispatch URL to the flight plan output so users can open SimBrief with origin, destination, aircraft type, and cruise altitude pre-filled.

**Architecture:** Add a `simbrief_url()` method on `FlightPlan` that formats the URL from existing fields. Print it at the end of the `generate()` output in `main.rs`. Unit test the URL method; extend the existing CLI integration test to verify the URL appears in output.

**Tech Stack:** Rust, no new dependencies

**Spec:** `docs/superpowers/specs/2026-03-21-simbrief-link-design.md`

---

### File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/flight_plan.rs` | Modify | Add `simbrief_url()` method on `FlightPlan` |
| `src/main.rs` | Modify | Print SimBrief URL after flight plan output |
| `tests/integration.rs` | Modify | Assert SimBrief URL appears in CLI output |

---

### Task 1: Unit test and implement `simbrief_url()` on FlightPlan

**Files:**
- Modify: `src/flight_plan.rs:101-178` (test module)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `src/flight_plan.rs`:

```rust
#[test]
fn simbrief_url_contains_correct_parameters() {
    let dep = find_by_icao("KJFK").expect("KJFK");
    let arr = find_by_icao("KLAX").expect("KLAX");
    let ac = aircraft_by_icao_type("B738").expect("B738");

    let fp = calculate_flight_plan(dep, arr, ac, taxi());
    let url = fp.simbrief_url();

    assert!(url.starts_with("https://dispatch.simbrief.com/options/custom?"),
        "unexpected base URL: {url}");
    assert!(url.contains("orig=KJFK"), "missing orig: {url}");
    assert!(url.contains("dest=KLAX"), "missing dest: {url}");
    assert!(url.contains("type=B738"), "missing type: {url}");
    assert!(url.contains(&format!("fl={}", fp.cruise_altitude_ft)),
        "missing or wrong fl: {url}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test simbrief_url_contains_correct_parameters --lib`
Expected: FAIL — `simbrief_url` method does not exist

- [ ] **Step 3: Write minimal implementation**

Add this method to `FlightPlan` in `src/flight_plan.rs`, right after the struct definition (after line 22):

```rust
impl FlightPlan {
    /// Build a SimBrief dispatch URL pre-filled with this flight plan's parameters.
    pub fn simbrief_url(&self) -> String {
        format!(
            "https://dispatch.simbrief.com/options/custom?orig={}&dest={}&type={}&fl={}",
            self.departure.icao,
            self.arrival.icao,
            self.aircraft.icao_type,
            self.cruise_altitude_ft,
        )
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test simbrief_url_contains_correct_parameters --lib`
Expected: PASS

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: No warnings

- [ ] **Step 6: Commit**

```bash
git add src/flight_plan.rs
git commit -m "feat: add simbrief_url() method to FlightPlan"
```

---

### Task 2: Print SimBrief URL in CLI output

**Files:**
- Modify: `src/main.rs:194` (after the Taxi println)

- [ ] **Step 1: Add the print lines**

In `src/main.rs`, inside the `Ok(fp)` arm of `generate()`, after the `Taxi` println (line 194), add:

```rust
            println!();
            println!("SimBrief:    {}", fp.simbrief_url());
```

- [ ] **Step 2: Run the CLI to verify output**

Run: `cargo run -- generate --aircraft B738 --time 4h`
Expected: Output ends with a blank line followed by `SimBrief:    https://dispatch.simbrief.com/options/custom?orig=...&dest=...&type=B738&fl=...`

- [ ] **Step 3: Run all tests to check nothing broke**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: print SimBrief dispatch URL after flight plan output"
```

---

### Task 3: Add integration test for SimBrief URL in CLI output

**Files:**
- Modify: `tests/integration.rs:74-85` (extend `cli_generate_produces_flight_plan`)

- [ ] **Step 1: Add assertion to existing CLI test**

In `tests/integration.rs`, in the `cli_generate_produces_flight_plan` test, add after the existing assertions (after line 84):

```rust
    assert!(stdout.contains("SimBrief:    https://dispatch.simbrief.com/options/custom?"),
        "expected SimBrief URL in output, got: {stdout}");
    assert!(stdout.contains("type=C172"), "expected type=C172 in SimBrief URL, got: {stdout}");
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test --test integration cli_generate_produces_flight_plan`
Expected: PASS

- [ ] **Step 3: Also verify the profile-based CLI test**

In `tests/integration.rs`, in the `cli_generate_with_profile` test, add after the existing assertions (after line 110):

```rust
    assert!(stdout.contains("SimBrief:    https://dispatch.simbrief.com/options/custom?"),
        "expected SimBrief URL in profile output, got: {stdout}");
```

- [ ] **Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: No warnings

- [ ] **Step 6: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for SimBrief URL output"
```
