ALTER TABLE users ALTER COLUMN password TYPE VARCHAR(255);
CREATE INDEX IF NOT EXISTS idx_users_user_name ON users(user_name);