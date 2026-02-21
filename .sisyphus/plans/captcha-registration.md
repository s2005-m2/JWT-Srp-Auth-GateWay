# CAPTCHA-Gated Registration Flow

## TL;DR

> **Quick Summary**: Add optional CAPTCHA verification to the registration flow, toggled by `ARC_AUTH__CAPTCHA__ENABLED=true` env var. When enabled, clients must first fetch a captcha image, then submit captcha answer with registration. When disabled, zero changes to existing behavior.
> 
> **Deliverables**:
> - `GET /auth/captcha` endpoint returning `{ captcha_id, image }` (base64 PNG)
> - Modified `POST /auth/register` accepting optional `captcha_id` + `captcha_text` fields (required only when CAPTCHA enabled)
> - `POST /auth/register/verify` unchanged
> - PostgreSQL migration for `captchas` table
> - `CaptchaService` for generation + validation
> - Config struct + env var toggle
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 2 waves
> **Critical Path**: Task 1 (config+migration+service) → Task 2 (handlers+routing) → Task 3 (cleanup+unwrap audit)

---

## Context

### Original Request
Add CAPTCHA to registration flow. When `CAPTCHA=true`, the flow becomes:
1. `GET /auth/captcha` → returns `{ captcha_id, image }` (base64)
2. `POST /auth/register` → requires `{ email, captcha_id, captcha_text }`
3. `POST /auth/register/verify` → unchanged `{ email, code, salt, verifier }`

When CAPTCHA is not enabled, the existing flow must have ZERO changes — production projects depend on it.

### Interview Summary
**Key Decisions**:
- Config: Env var only (`ARC_AUTH__CAPTCHA__ENABLED=true`), mapped via existing `config` crate pattern
- CAPTCHA generation: Self-generated via `captcha` Rust crate
- Storage: PostgreSQL `captchas` table
- TTL: 60 seconds
- Case-insensitive text comparison
- Single-use: captcha burned on first attempt (right or wrong)
- Default: CAPTCHA disabled (backward compatible)

### Metis Review
**Identified Gaps** (addressed):
- Case sensitivity: Resolved → case-insensitive comparison (lowercase both sides)
- Single-use policy: Resolved → burn on attempt via atomic `UPDATE ... WHERE used = false RETURNING text`
- Captcha crate choice: Use `captcha` crate (simple, sufficient for this use case)

---

## Work Objectives

### Core Objective
Add a feature-flagged CAPTCHA step to the registration flow that is completely invisible when disabled.

### Concrete Deliverables
- Migration `014_captchas.sql`
- `CaptchaConfig` in `config.rs`
- `CaptchaService` in `services/captcha.rs`
- `GET /auth/captcha` handler in `handlers/captcha.rs`
- Modified `POST /auth/register` handler with conditional captcha validation
- `captcha` crate added to `Cargo.toml`
- Cleanup of expired captchas in existing scheduler
- New `AppError::InvalidCaptcha` variant

### Definition of Done
- [ ] `cargo check` passes
- [ ] `cargo test` passes (existing tests unbroken)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] When `ARC_AUTH__CAPTCHA__ENABLED` is unset/false: `POST /auth/register {"email":"..."}` works exactly as before
- [ ] When enabled: `GET /auth/captcha` returns valid JSON with `captcha_id` + `image` (base64)
- [ ] When enabled: `POST /auth/register` without captcha fields returns error
- [ ] When enabled: `POST /auth/register` with correct captcha proceeds to send verification code

### Must Have
- Feature toggle via env var, default OFF
- Zero behavioral change when disabled
- 60-second captcha TTL
- Case-insensitive comparison
- Single-use (burned on attempt)
- Base64 PNG image in response

### Must NOT Have (Guardrails)
- NO changes to `POST /auth/register/verify` endpoint
- NO changes to login flow (`/auth/login/*`)
- NO changes to admin API
- NO Redis or new infrastructure dependencies
- NO breaking changes to existing `RegisterRequest` when CAPTCHA disabled
- NO `unwrap()` in handler/service code
- NO over-abstraction — minimal code only

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: YES (`cargo test`)
- **Automated tests**: Tests-after (unit tests for captcha service)
- **Framework**: `cargo test` (built-in)

### QA Policy
Every task includes agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **API/Backend**: Use Bash (curl) — Send requests, assert status + response fields
- **Library/Module**: Use Bash (cargo test) — Run tests, verify pass

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — all independent):
├── Task 1: Config struct + env var toggle [quick]
├── Task 2: Migration + CaptchaService [unspecified-high]
├── Task 3: AppError::InvalidCaptcha variant [quick]

Wave 2 (Handlers — depends on Wave 1):
├── Task 4: GET /auth/captcha handler + route [unspecified-high]
├── Task 5: Modify POST /auth/register for conditional captcha [deep]

Wave 3 (Integration — depends on Wave 2):
├── Task 6: Wire into AppState + main.rs + cleanup scheduler [unspecified-high]
├── Task 7: Unwrap audit + cargo check/clippy/test [quick]

Wave FINAL (Verification):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real QA via curl (unspecified-high)
├── Task F4: Scope fidelity check (deep)

Critical Path: T1+T2+T3 → T4+T5 → T6 → T7 → F1-F4
Max Concurrent: 3 (Wave 1)
```

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 1 | — | 4, 5, 6 |
| 2 | — | 4, 5, 6 |
| 3 | — | 4, 5 |
| 4 | 1, 2, 3 | 6 |
| 5 | 1, 2, 3 | 6 |
| 6 | 4, 5 | 7 |
| 7 | 6 | F1-F4 |

### Agent Dispatch Summary

- **Wave 1**: 3 tasks — T1 → `quick`, T2 → `unspecified-high`, T3 → `quick`
- **Wave 2**: 2 tasks — T4 → `unspecified-high`, T5 → `deep`
- **Wave 3**: 2 tasks — T6 → `unspecified-high`, T7 → `quick`
- **FINAL**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Config struct + env var toggle

  **What to do**:
  - Add `CaptchaConfig` struct to `src/config.rs` with `enabled: bool` (default `false`)
  - Add `#[serde(default)]` field `pub captcha: CaptchaConfig` to `AppConfig`
  - This enables `ARC_AUTH__CAPTCHA__ENABLED=true` env var via existing config crate pattern
  - Implement `Default` for `CaptchaConfig` returning `enabled: false`

  **Must NOT do**:
  - Do NOT add TOML sections to `config/default.toml` (env-only toggle)
  - Do NOT add any fields beyond `enabled: bool`

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: Tasks 4, 5, 6
  - **Blocked By**: None

  **References**:
  - `src/config.rs:4-12` — `AppConfig` struct, follow the `#[serde(default)]` pattern used by `routing: RoutesConfig`
  - `src/config.rs:52-56` — `RoutesConfig` with `Default` derive, follow same pattern
  - `src/config.rs:81-93` — `AppConfig::load()` showing env prefix `ARC_AUTH__` with `__` separator

  **Acceptance Criteria**:

  ```
  Scenario: Config defaults to disabled
    Tool: Bash (cargo test)
    Steps:
      1. Add unit test: `AppConfig` deserialized from empty config has `captcha.enabled == false`
      2. Run `cargo test test_captcha_config_default`
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-1-config-default.txt

  Scenario: Env var enables captcha
    Tool: Bash
    Steps:
      1. Run `ARC_AUTH__CAPTCHA__ENABLED=true cargo test test_captcha_config_enabled` (or verify in integration)
    Expected Result: `captcha.enabled == true`
    Evidence: .sisyphus/evidence/task-1-config-enabled.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(captcha): add config, migration, service, and error variant`
  - Files: `src/config.rs`

- [x] 2. Migration + CaptchaService

  **What to do**:
  - Create `migrations/014_captchas.sql`:
    ```sql
    CREATE TABLE captchas (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        text VARCHAR(10) NOT NULL,
        used BOOLEAN NOT NULL DEFAULT FALSE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '60 seconds'
    );
    CREATE INDEX idx_captchas_expires_at ON captchas (expires_at);
    ```
  - Create `src/services/captcha.rs` with `CaptchaService`:
    - `new(db_pool: Arc<PgPool>)` constructor
    - `async fn generate(&self) -> Result<(String, String)>` — generates captcha using `captcha` crate, stores in DB, returns `(captcha_id, base64_image)`
    - `async fn validate(&self, captcha_id: &str, text: &str) -> Result<()>` — atomic `UPDATE captchas SET used = true WHERE id = $1 AND used = false AND expires_at > NOW() RETURNING text`, then case-insensitive compare. Returns `AppError::InvalidCaptcha` on failure.
  - Register module in `src/services/mod.rs`: add `pub mod captcha;` and `pub use captcha::CaptchaService;`
  - Add `captcha` crate to `Cargo.toml` dependencies
  - Add `base64` crate to `Cargo.toml` dependencies (for encoding PNG to base64)

  **Must NOT do**:
  - Do NOT use `unwrap()` — use `?` and `map_err` throughout
  - Do NOT store the raw captcha text in logs
  - Do NOT add unnecessary abstraction layers

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Tasks 4, 5, 6
  - **Blocked By**: None

  **References**:
  - `src/services/srp.rs` — Follow the service struct pattern: `struct XService { db_pool: Arc<PgPool> }` + `impl XService { pub fn new(...) }`
  - `src/services/mod.rs` — Registration pattern: `pub mod x; pub use x::XService;`
  - `src/api/handlers/register.rs:145-161` — `save_verification_code()` for DB insert pattern with `sqlx::query`
  - `src/api/handlers/verify.rs:32-42` — Atomic SELECT + FOR UPDATE pattern for concurrent-safe reads
  - `src/error.rs:8-55` — `AppError` enum for error type reference
  - `Cargo.toml:56-60` — Existing utils deps section for where to add `captcha` and `base64`

  **Acceptance Criteria**:

  ```
  Scenario: CaptchaService generates valid captcha
    Tool: Bash (cargo test)
    Steps:
      1. Write unit test that calls `CaptchaService::generate()` (requires test DB or mock)
      2. Verify returned tuple has non-empty captcha_id (valid UUID) and non-empty base64 string
      3. Run `cargo test captcha`
    Expected Result: Test passes, captcha_id is UUID, image is valid base64
    Evidence: .sisyphus/evidence/task-2-generate.txt

  Scenario: Migration file is valid SQL
    Tool: Bash
    Steps:
      1. Verify `migrations/014_captchas.sql` exists
      2. Run `cargo check` to ensure sqlx migration macro picks it up
    Expected Result: No compilation errors
    Evidence: .sisyphus/evidence/task-2-migration.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(captcha): add config, migration, service, and error variant`
  - Files: `migrations/014_captchas.sql`, `src/services/captcha.rs`, `src/services/mod.rs`, `Cargo.toml`

- [x] 3. AppError::InvalidCaptcha variant

  **What to do**:
  - Add `InvalidCaptcha` variant to `AppError` enum in `src/error.rs`
  - `#[error("Invalid captcha")]` with `InvalidCaptcha`
  - Map to `StatusCode::BAD_REQUEST` in `status_code()`
  - Map to `"INVALID_CAPTCHA"` in `error_code()`
  - Add `tracing::info!` log in `log_error()`

  **Must NOT do**:
  - Do NOT modify any existing error variants
  - Do NOT change existing status code mappings

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: Tasks 4, 5
  - **Blocked By**: None

  **References**:
  - `src/error.rs:8-55` — `AppError` enum, add variant following existing pattern
  - `src/error.rs:70-86` — `status_code()` match, add `Self::InvalidCaptcha => StatusCode::BAD_REQUEST`
  - `src/error.rs:88-105` — `error_code()` match, add `Self::InvalidCaptcha => "INVALID_CAPTCHA"`
  - `src/error.rs:124-199` — `log_error()` match, add info-level log like `InvalidCode` pattern at line 166-170

  **Acceptance Criteria**:

  ```
  Scenario: Existing error tests still pass
    Tool: Bash
    Steps:
      1. Run `cargo test error`
    Expected Result: All existing error tests pass, zero regressions
    Evidence: .sisyphus/evidence/task-3-error-tests.txt

  Scenario: New variant compiles correctly
    Tool: Bash
    Steps:
      1. Run `cargo check`
    Expected Result: No compilation errors
    Evidence: .sisyphus/evidence/task-3-check.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(captcha): add config, migration, service, and error variant`
  - Files: `src/error.rs`

- [x] 4. GET /auth/captcha handler + route

  **What to do**:
  - Create `src/api/handlers/captcha.rs` with:
    - `CaptchaResponse { captcha_id: String, image: String }` (Serialize)
    - `async fn get_captcha(State(state): State<AppState>) -> Result<Json<CaptchaResponse>>` — calls `state.captcha_service.generate()`, returns JSON
    - If CAPTCHA is not enabled, return `AppError::NotFound`
  - Register in `src/api/handlers/mod.rs`: add `pub mod captcha;` and `pub use captcha::get_captcha;`
  - Add route in `src/api/mod.rs` `create_auth_router()`: `.route("/captcha", get(handlers::get_captcha))` inside `auth_routes`

  **Must NOT do**:
  - Do NOT add any routes outside `/auth/captcha`
  - Do NOT modify any existing routes
  - Do NOT add rate limiting beyond what `auth_limiter` already provides

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 5)
  - **Blocks**: Task 6
  - **Blocked By**: Tasks 1, 2, 3

  **References**:
  - `src/api/handlers/register.rs:18-43` — Handler pattern: `State(state): State<AppState>`, `Json(req)`, return `Result<Json<...>>`
  - `src/api/handlers/mod.rs` — Module registration pattern: `pub mod x; pub use x::func_name;`
  - `src/api/mod.rs:37-47` — `auth_routes` Router where new route goes, follow `.route(...)` pattern
  - `src/api/mod.rs:1` — Import `get` from axum routing (already imported)

  **Acceptance Criteria**:

  ```
  Scenario: GET /auth/captcha returns valid response when enabled
    Tool: Bash (curl)
    Preconditions: Server running with ARC_AUTH__CAPTCHA__ENABLED=true
    Steps:
      1. curl -s http://localhost:8080/auth/captcha
      2. Parse JSON response
      3. Assert response has "captcha_id" field (UUID format)
      4. Assert response has "image" field (non-empty string starting with base64 data)
    Expected Result: HTTP 200, JSON with captcha_id and image fields
    Failure Indicators: HTTP 404, missing fields, empty image
    Evidence: .sisyphus/evidence/task-4-captcha-get.txt

  Scenario: GET /auth/captcha returns 404 when disabled
    Tool: Bash (curl)
    Preconditions: Server running WITHOUT ARC_AUTH__CAPTCHA__ENABLED
    Steps:
      1. curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/auth/captcha
    Expected Result: HTTP 404
    Evidence: .sisyphus/evidence/task-4-captcha-disabled.txt
  ```

  **Commit**: YES (groups with Wave 2+3)
  - Message: `feat(captcha): add captcha endpoint and conditional registration flow`
  - Files: `src/api/handlers/captcha.rs`, `src/api/handlers/mod.rs`, `src/api/mod.rs`

- [x] 5. Modify POST /auth/register for conditional captcha validation

  **What to do**:
  - Modify `src/api/handlers/register.rs`:
    - Add `CaptchaService` to imports (via `crate::services::CaptchaService` or through AppState)
    - Add optional fields to `RegisterRequest`: `captcha_id: Option<String>`, `captcha_text: Option<String>`
    - In `register()` handler, BEFORE existing logic:
      - Read `state.captcha_enabled` (bool from config, stored in AppState)
      - If captcha enabled: require both `captcha_id` and `captcha_text` present (else `AppError::InvalidCaptcha`), then call `state.captcha_service.validate(captcha_id, captcha_text).await?`
      - If captcha NOT enabled: skip entirely, existing flow runs unchanged
    - The rest of the handler (email validation, user check, code generation, email send) stays EXACTLY as-is

  **Must NOT do**:
  - Do NOT change the response type `RegisterResponse`
  - Do NOT modify `is_valid_email()`, `generate_code()`, `save_verification_code()`
  - Do NOT touch `verify.rs` at all
  - Do NOT change behavior when captcha is disabled — `RegisterRequest` with only `email` must still work

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 4)
  - **Blocks**: Task 6
  - **Blocked By**: Tasks 1, 2, 3

  **References**:
  - `src/api/handlers/register.rs:1-43` — Current `register()` handler, the captcha check goes BEFORE line 24 (`if !is_valid_email`)
  - `src/api/handlers/register.rs:8-11` — `RegisterRequest` struct, add optional fields here
  - `src/api/handlers/verify.rs:32-42` — Pattern for atomic DB check (similar to captcha validate)
  - `src/error.rs` — `AppError::InvalidCaptcha` (from Task 3)

  **Acceptance Criteria**:

  ```
  Scenario: Register works without captcha when disabled (backward compat)
    Tool: Bash (curl)
    Preconditions: Server running WITHOUT ARC_AUTH__CAPTCHA__ENABLED
    Steps:
      1. curl -s -X POST http://localhost:8080/auth/register -H "Content-Type: application/json" -d '{"email":"test@gmail.com"}'
      2. Assert HTTP status is NOT 400 with INVALID_CAPTCHA error
    Expected Result: Normal registration flow (200 or email-related error, NOT captcha error)
    Failure Indicators: HTTP 400 with "INVALID_CAPTCHA" code
    Evidence: .sisyphus/evidence/task-5-register-no-captcha.txt

  Scenario: Register requires captcha when enabled
    Tool: Bash (curl)
    Preconditions: Server running with ARC_AUTH__CAPTCHA__ENABLED=true
    Steps:
      1. curl -s -X POST http://localhost:8080/auth/register -H "Content-Type: application/json" -d '{"email":"test@gmail.com"}'
      2. Assert HTTP 400 with "INVALID_CAPTCHA" error code
    Expected Result: Rejected — captcha fields missing
    Evidence: .sisyphus/evidence/task-5-register-missing-captcha.txt

  Scenario: Register with valid captcha proceeds normally
    Tool: Bash (curl)
    Preconditions: Server running with ARC_AUTH__CAPTCHA__ENABLED=true
    Steps:
      1. GET /auth/captcha → extract captcha_id
      2. (Read captcha text from DB for testing: SELECT text FROM captchas WHERE id = '<captcha_id>')
      3. POST /auth/register with {"email":"test@gmail.com","captcha_id":"<id>","captcha_text":"<text>"}
      4. Assert success response (verification code sent)
    Expected Result: HTTP 200, "Verification code sent"
    Evidence: .sisyphus/evidence/task-5-register-valid-captcha.txt

  Scenario: Register with wrong captcha text is rejected
    Tool: Bash (curl)
    Preconditions: Server running with ARC_AUTH__CAPTCHA__ENABLED=true
    Steps:
      1. GET /auth/captcha → extract captcha_id
      2. POST /auth/register with {"email":"test@gmail.com","captcha_id":"<id>","captcha_text":"WRONG"}
      3. Assert HTTP 400 with "INVALID_CAPTCHA"
    Expected Result: Rejected with INVALID_CAPTCHA
    Evidence: .sisyphus/evidence/task-5-register-wrong-captcha.txt

  Scenario: Reusing burned captcha is rejected
    Tool: Bash (curl)
    Steps:
      1. GET /auth/captcha → extract captcha_id
      2. POST /auth/register with wrong text (burns the captcha)
      3. POST /auth/register again with same captcha_id and correct text
      4. Assert HTTP 400 with "INVALID_CAPTCHA"
    Expected Result: Second attempt rejected — captcha already used
    Evidence: .sisyphus/evidence/task-5-register-reused-captcha.txt
  ```

  **Commit**: YES (groups with Wave 2+3)
  - Message: `feat(captcha): add captcha endpoint and conditional registration flow`
  - Files: `src/api/handlers/register.rs`

- [x] 6. Wire into AppState + main.rs + cleanup scheduler

  **What to do**:
  - Add `captcha_service: Arc<CaptchaService>` to `AppState` in `src/api/mod.rs`
  - Add `captcha_enabled: bool` to `AppState` in `src/api/mod.rs`
  - In `src/main.rs`:
    - Import `CaptchaService` from services
    - Create `let captcha_service = Arc::new(CaptchaService::new(db_pool.clone()));`
    - Add `captcha_enabled: config.captcha.enabled` and `captcha_service` to `AppState` construction
  - In `database_cleanup_scheduler()` in `src/main.rs`:
    - Add cleanup query: `DELETE FROM captchas WHERE expires_at < NOW() OR used = TRUE`
    - Follow exact pattern of existing `deleted_codes` block

  **Must NOT do**:
  - Do NOT change any existing service initialization order
  - Do NOT add new scheduler tasks — reuse existing `database_cleanup_scheduler`

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential after Wave 2)
  - **Blocks**: Task 7
  - **Blocked By**: Tasks 4, 5

  **References**:
  - `src/api/mod.rs:16-30` — `AppState` struct, add fields following existing pattern
  - `src/main.rs:46-50` — Service creation pattern: `Arc::new(XService::new(db_pool.clone()))`
  - `src/main.rs:91-104` — `AppState` construction, add new fields here
  - `src/main.rs:238-274` — `database_cleanup_scheduler()`, add captcha cleanup after refresh_tokens block (line ~272)
  - `src/main.rs:11` — `use services::{...}` import line, add `CaptchaService`

  **Acceptance Criteria**:

  ```
  Scenario: Project compiles with all wiring
    Tool: Bash
    Steps:
      1. Run `cargo check`
    Expected Result: No compilation errors
    Evidence: .sisyphus/evidence/task-6-check.txt

  Scenario: Cleanup scheduler handles captchas
    Tool: Bash
    Steps:
      1. Verify `database_cleanup_scheduler` contains captcha cleanup query by reading the file
    Expected Result: DELETE FROM captchas query present
    Evidence: .sisyphus/evidence/task-6-cleanup.txt
  ```

  **Commit**: YES (groups with Wave 2+3)
  - Message: `feat(captcha): add captcha endpoint and conditional registration flow`
  - Files: `src/api/mod.rs`, `src/main.rs`

- [x] 7. Unwrap audit + cargo check/clippy/test

  **What to do**:
  - Search ALL new/changed `.rs` files for `.unwrap()` and `.expect()`
  - Replace each with proper error handling (`?`, `map_err`, `unwrap_or_else`)
  - Add `tracing::warn!` or `tracing::error!` at recovery points
  - Run `cargo check` — must pass
  - Run `cargo clippy -- -D warnings` — must pass
  - Run `cargo test` — all existing + new tests must pass
  - Run `cargo fmt --check` — must pass

  **Must NOT do**:
  - Do NOT modify any files not touched by this feature
  - Do NOT suppress clippy warnings with `#[allow(...)]`

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (after Task 6)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 6

  **References**:
  - `AGENTS.md` — "WORKFLOW RULE: UNWRAP AUDIT" section — mandatory for every task
  - All files from Tasks 1-6: `config.rs`, `error.rs`, `services/captcha.rs`, `handlers/captcha.rs`, `handlers/register.rs`, `api/mod.rs`, `main.rs`

  **Acceptance Criteria**:

  ```
  Scenario: Zero unwrap() in new code
    Tool: Bash (grep)
    Steps:
      1. grep -n "unwrap()\|\.expect(" src/services/captcha.rs src/api/handlers/captcha.rs
      2. Assert zero matches
    Expected Result: No unwrap() or expect() found
    Evidence: .sisyphus/evidence/task-7-unwrap-audit.txt

  Scenario: Full build pipeline passes
    Tool: Bash
    Steps:
      1. cargo fmt --check
      2. cargo check
      3. cargo clippy -- -D warnings
      4. cargo test
    Expected Result: All four commands exit 0
    Evidence: .sisyphus/evidence/task-7-build-pipeline.txt
  ```

  **Commit**: NO (amend previous if needed, or no separate commit)

---

## Final Verification Wave

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files exist. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo check`, `cargo clippy -- -D warnings`, `cargo test`. Review all changed files for: `unwrap()`, empty catches, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [ ] F3. **Real QA via curl** — `unspecified-high`
  Start the server. Test CAPTCHA disabled flow (default): `POST /auth/register {"email":"..."}` works as before. Enable CAPTCHA, restart. Test `GET /auth/captcha` returns valid response. Test `POST /auth/register` with/without captcha fields. Test expired captcha (wait 60s). Test wrong captcha text. Save evidence.
  Output: `Scenarios [N/N pass] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify nothing beyond spec was built. Check verify.rs, srp_login.rs, admin routes are UNTOUCHED. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Wave 1**: `feat(captcha): add config, migration, service, and error variant` — config.rs, services/captcha.rs, migrations/014_captchas.sql, error.rs, Cargo.toml
- **Wave 2+3**: `feat(captcha): add captcha endpoint and conditional registration flow` — handlers/captcha.rs, handlers/register.rs, handlers/mod.rs, api/mod.rs, main.rs, services/mod.rs

---

## Success Criteria

### Verification Commands
```bash
cargo check          # Expected: no errors
cargo clippy -- -D warnings  # Expected: no warnings
cargo test           # Expected: all tests pass
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All existing tests pass
- [ ] CAPTCHA disabled: existing flow unchanged
- [ ] CAPTCHA enabled: new 3-step flow works
