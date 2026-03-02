# Sprint 095: Anti-Fake Engineering and Fake-Source Purge

## Metadata

- Sprint ID: 95
- Status: done
- Gate: G42
- Owner: Backend + Security + QA
- Dependencies: 94

## Objective

Close remaining production gaps in synthetic/fake-data handling by engineering a full anti-fake pipeline: enumerate and purge fake sources, harden token validation paths, and add continuous verification coverage so fake artifacts cannot reappear in hot paths.

## Scope In

1. Audit all remaining fake/mock/stub data touchpoints in auth/session/security verification and API runtime.
2. Define and persist a canonical anti-fake source-list (deny/allow policy) for automated filtering.
3. Extend runtime anti-fake checks for replay resistance, malformed token handling, and edge-case telemetry visibility.
4. Add test coverage for fake-source purge and anti-fake regression scenarios.
5. Add operational docs runbook updates for anti-fake tuning and incident triage.

## Scope Out

1. New product feature work unrelated to trust/security hardening.
2. New machine-learning model training or external threat-intel integrations.
3. UI redesign or non-security non-functional cleanup.

## Work Packages

1. **WP-95A:** Runtime anti-fake inventory sweep and fake-source mapping with ownership assignments.
2. **WP-95B:** Build or complete canonical fake-source registry and consume it in anti-fake request validation.
3. **WP-95C:** Expand anti-fake guards for malformed input/attack-like patterns and ensure consistent rejection responses.
4. **WP-95D:** Add automated regression tests and evidence for zero-fake-path behavior on critical login/session operations.
5. **WP-95E:** Update security/readiness docs and operational checklists for ongoing anti-fake maintenance.

## Acceptance Criteria

1. All fake/mock/stub data sources used outside explicit local-test scopes are removed from production and enterprise runtime paths.
2. Anti-fake policy is centralized and documented, with deterministic defaults in configuration examples.
3. Critical anti-fake validation paths fail safely on malformed/abusive payloads and emit actionable telemetry.
4. Regression test suite includes anti-fake cases for:
   - fake/nonce mismatch
   - stale/replayed challenges
   - invalid timestamps
   - trust-proxy and token/IP mismatch cases
5. Sprint 95 evidence can be reproduced via documented verification commands.

## Verification Commands

```bash
cargo test --locked --lib anti_fake
cargo test --locked --lib anti_fake::tests:: -- --nocapture
rg -n "TODO|FIXME|unimplemented!|panic!(\"todo\"|fake\\b" src
```

## Gate Condition

G42 passes when anti-fake hardening and fake-source purge actions are implemented, validated by regression evidence, and operational procedures are updated for production monitoring.
