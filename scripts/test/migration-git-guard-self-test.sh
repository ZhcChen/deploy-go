#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
GUARD="$ROOT_DIR/scripts/test/migration-git-guard.sh"
HOOK="$ROOT_DIR/.githooks/pre-commit"
TMP_DIR=$(mktemp -d)
FAKE_BIN="$TMP_DIR/bin"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

fail() {
  echo "migration Git guard self-test: $*" >&2
  exit 1
}

mkdir -p "$FAKE_BIN"
cat >"$FAKE_BIN/go" <<'GO'
#!/usr/bin/env bash
echo 'unexpected go invocation from staged migration guard' >&2
exit 91
GO
chmod +x "$FAKE_BIN/go"
cat >"$FAKE_BIN/make" <<'MAKE'
#!/usr/bin/env bash
echo 'unexpected make invocation from staged migration hook' >&2
exit 92
MAKE
chmod +x "$FAKE_BIN/make"

write_valid_migration() {
  local path=$1 table=${2:-fixture_items}
  mkdir -p "$(dirname "$path")"
  cat >"$path" <<SQL
CREATE TABLE ${table} (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
SQL
}

write_destructive_migration() {
  local path=$1 operation=$2
  mkdir -p "$(dirname "$path")"
  cat >"$path" <<SQL
ALTER TABLE fixture_items
    ${operation};
SQL
}

fixture_guard_path() {
  git rev-parse --git-path 'deploy-go-tools/migration-git-guard/v1'
}

new_repo() {
  local name repo
  name=$1
  repo="$TMP_DIR/$name"
  git init -q "$repo"
  (
    cd "$repo"
    git config user.name 'migration-guard-test'
    git config user.email 'migration-guard-test@example.invalid'
    git config core.autocrlf false
    write_valid_migration api/migrations/0001_baseline_fixture.sql baseline_fixture
    printf 'baseline\n' > README.md
    mkdir -p .githooks scripts/test
    cp "$HOOK" .githooks/pre-commit
    chmod +x .githooks/pre-commit
    cp "$GUARD" scripts/test/migration-git-guard.sh
    chmod +x scripts/test/migration-git-guard.sh
    git add .
    git commit -qm baseline
    git config core.hooksPath .githooks
    local_guard=$(fixture_guard_path)
    mkdir -p "$local_guard"
    cp "$GUARD" "$local_guard/migration-git-guard.sh"
    chmod +x "$local_guard/migration-git-guard.sh"
  )
  printf '%s\n' "$repo"
}

expect_commit_passes() {
  local message=$1 log
  log=$(mktemp "$TMP_DIR/pass.XXXXXX")
  if ! PATH="$FAKE_BIN:$PATH" git commit -qm "$message" >"$log" 2>&1; then
    cat "$log" >&2
    fail "expected commit to pass: $message"
  fi
}

expect_commit_fails() {
  local message=$1 log
  log=$(mktemp "$TMP_DIR/fail.XXXXXX")
  if PATH="$FAKE_BIN:$PATH" git commit -qm "$message" >"$log" 2>&1; then
    cat "$log" >&2
    fail "expected commit to fail: $message"
  fi
  grep -q 'migration Git guard' "$log" || {
    cat "$log" >&2
    fail "failed commit did not execute migration guard: $message"
  }
}

test_non_migration_commit() {
  local repo
  repo=$(new_repo non-migration)
  (
    cd "$repo"
    printf 'ordinary change\n' >> README.md
    git add README.md
    expect_commit_passes ordinary-change
  )
}

test_valid_new_migration() {
  local repo
  repo=$(new_repo valid-new)
  (
    cd "$repo"
    write_valid_migration api/migrations/0002_add_fixture_two.sql fixture_two
    git add api/migrations
    expect_commit_passes valid-new-migration
  )
}

test_staged_content_is_authoritative() {
  local repo staged_path
  repo=$(new_repo staged-content)
  staged_path=api/migrations/0002_staged_content.sql
  (
    cd "$repo"
    write_valid_migration "$staged_path" staged_content
    git add "$staged_path"
    write_destructive_migration "$staged_path" 'DROP COLUMN legacy_value'
    expect_commit_passes staged-content-wins
    git show "HEAD:${staged_path}" | grep -q 'CREATE TABLE staged_content' || fail 'committed migration did not use staged content'
  )
}

test_bad_staged_content_is_rejected() {
  local repo staged_path
  repo=$(new_repo bad-staged-content)
  staged_path=api/migrations/0002_bad_staged_content.sql
  (
    cd "$repo"
    write_destructive_migration "$staged_path" 'DROP COLUMN legacy_value'
    git add "$staged_path"
    write_valid_migration "$staged_path" repaired_worktree_only
    expect_commit_fails bad-staged-content
  )
}

test_history_and_shape_rejections() {
  local repo

  repo=$(new_repo history-modification)
  (
    cd "$repo"
    printf '\n-- edited\n' >> api/migrations/0001_baseline_fixture.sql
    git add api/migrations/0001_baseline_fixture.sql
    expect_commit_fails history-modification
  )

  repo=$(new_repo deletion)
  (
    cd "$repo"
    git rm -q api/migrations/0001_baseline_fixture.sql
    expect_commit_fails migration-deletion
  )

  repo=$(new_repo rename)
  (
    cd "$repo"
    git mv api/migrations/0001_baseline_fixture.sql api/migrations/0002_renamed_fixture.sql
    expect_commit_fails migration-rename
  )

  repo=$(new_repo copy)
  (
    cd "$repo"
    cp api/migrations/0001_baseline_fixture.sql api/migrations/0002_copied_fixture.sql
    git add api/migrations/0002_copied_fixture.sql
    expect_commit_fails migration-copy
  )

  repo=$(new_repo mode)
  (
    cd "$repo"
    write_valid_migration api/migrations/0002_executable_fixture.sql executable_fixture
    git add api/migrations/0002_executable_fixture.sql
    git update-index --chmod=+x api/migrations/0002_executable_fixture.sql
    expect_commit_fails migration-mode
  )

  repo=$(new_repo invalid-name)
  (
    cd "$repo"
    write_valid_migration api/migrations/not-a-migration.sql invalid_name
    git add api/migrations/not-a-migration.sql
    expect_commit_fails invalid-name
  )

  repo=$(new_repo nested)
  (
    cd "$repo"
    write_valid_migration api/migrations/nested/0002_nested.sql nested_fixture
    git add api/migrations/nested/0002_nested.sql
    expect_commit_fails nested-migration
  )

  repo=$(new_repo non-sql)
  (
    cd "$repo"
    printf 'not sql\n' > api/migrations/0002_notes.txt
    git add api/migrations/0002_notes.txt
    expect_commit_fails non-sql-migration
  )
}

test_version_rejections() {
  local repo

  repo=$(new_repo old-version)
  (
    cd "$repo"
    write_valid_migration api/migrations/0001_reused_version.sql reused_version
    git add api/migrations/0001_reused_version.sql
    expect_commit_fails old-version
  )

  repo=$(new_repo duplicate-version)
  (
    cd "$repo"
    write_valid_migration api/migrations/0002_first.sql duplicate_first
    write_valid_migration api/migrations/0002_second.sql duplicate_second
    git add api/migrations/0002_first.sql api/migrations/0002_second.sql
    expect_commit_fails duplicate-version
  )
}

test_destructive_rejections() {
  local repo

  repo=$(new_repo drop-column)
  (
    cd "$repo"
    write_destructive_migration api/migrations/0002_drop_legacy_column.sql 'DROP COLUMN legacy_value'
    git add api/migrations/0002_drop_legacy_column.sql
    expect_commit_fails drop-column
  )

  repo=$(new_repo drop-table)
  (
    cd "$repo"
    cat > api/migrations/0002_drop_legacy_table.sql <<'SQL'
DROP
TABLE legacy_items;
SQL
    git add api/migrations/0002_drop_legacy_table.sql
    expect_commit_fails drop-table
  )

  repo=$(new_repo commented-drop)
  (
    cd "$repo"
    cat > api/migrations/0002_keep_deprecated_column.sql <<'SQL'
-- DROP COLUMN appears only in documentation and must not trigger the guard.
COMMENT ON COLUMN fixture_items.legacy_value IS 'deprecated: retained for compatibility';
/* DROP TABLE legacy_items; */
SQL
    git add api/migrations/0002_keep_deprecated_column.sql
    expect_commit_passes commented-drop
  )

  repo=$(new_repo string-drop)
  (
    cd "$repo"
    cat > api/migrations/0002_note_mentions_drop.sql <<'SQL'
INSERT INTO fixture_items (id, note) VALUES (1, 'DROP TABLE legacy_items must not trigger the guard.');
SQL
    git add api/migrations/0002_note_mentions_drop.sql
    expect_commit_passes string-drop
  )
}

test_installed_policy_ignores_worktree_tampering() {
  local repo staged_path
  repo=$(new_repo worktree-policy-tampering)
  staged_path=api/migrations/0002_bad_with_tampering.sql
  (
    cd "$repo"
    write_destructive_migration "$staged_path" 'DROP COLUMN legacy_value'
    git add "$staged_path"
    mkdir -p scripts/test
    printf '#!/usr/bin/env bash\nexit 0\n' > scripts/test/migration-git-guard.sh
    printf 'migration-git-guard-staged:\n\t@true\n' > Makefile
    expect_commit_fails installed-policy-ignores-worktree-tampering
  )
}

test_stale_installed_guard_is_rejected() {
  local repo staged_path local_guard log
  repo=$(new_repo stale-installed-guard)
  staged_path=api/migrations/0002_valid_with_stale_guard.sql
  (
    cd "$repo"
    write_valid_migration "$staged_path" valid_with_stale_guard
    git add "$staged_path"
    local_guard=$(fixture_guard_path)
    printf '#!/usr/bin/env bash\nexit 0\n' > "$local_guard/migration-git-guard.sh"
    log=$(mktemp "$TMP_DIR/stale.XXXXXX")
    if PATH="$FAKE_BIN:$PATH" git commit -qm stale-installed-guard >"$log" 2>&1; then
      cat "$log" >&2
      fail 'expected stale installed guard to fail commit'
    fi
    grep -q 'does not match committed source' "$log" || {
      cat "$log" >&2
      fail 'stale installed guard did not report the mismatch'
    }
  )
}

test_worktree_precheck() {
  local repo log

  repo=$(new_repo worktree-valid)
  (
    cd "$repo"
    write_valid_migration api/migrations/0002_worktree_valid.sql worktree_valid
    PATH="$FAKE_BIN:$PATH" bash "$GUARD" --worktree
  )

  repo=$(new_repo worktree-invalid)
  log="$TMP_DIR/worktree-invalid.log"
  (
    cd "$repo"
    write_destructive_migration api/migrations/0002_worktree_invalid.sql 'DROP COLUMN legacy_value'
    if PATH="$FAKE_BIN:$PATH" bash "$GUARD" --worktree >"$log" 2>&1; then
      cat "$log" >&2
      fail 'expected invalid worktree migration to fail'
    fi
  )
  grep -q 'migration Git guard' "$log" || {
    cat "$log" >&2
    fail 'invalid worktree migration did not execute guard'
  }
}

test_setup_and_verify() {
  local repo
  repo=$(new_repo setup-and-verify)
  (
    cd "$repo"
    git config --local --unset core.hooksPath
    rm -rf "$(git rev-parse --git-path deploy-go-tools/migration-git-guard/v1)"
    bash "$GUARD" --setup
    [[ "$(git config --local --get core.hooksPath)" == '.githooks' ]] || fail 'setup did not configure core.hooksPath'
    bash "$GUARD" --verify
  )
}

test_non_migration_commit
test_valid_new_migration
test_staged_content_is_authoritative
test_bad_staged_content_is_rejected
test_history_and_shape_rejections
test_version_rejections
test_destructive_rejections
test_installed_policy_ignores_worktree_tampering
test_stale_installed_guard_is_rejected
test_worktree_precheck
test_setup_and_verify

echo 'migration Git guard self-test passed.'
