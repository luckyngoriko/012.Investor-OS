-- Sprint 102: Account lockout columns
ALTER TABLE auth_users
    ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN locked_until TIMESTAMPTZ;
