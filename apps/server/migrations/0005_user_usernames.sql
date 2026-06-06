-- Add username column to users
ALTER TABLE users ADD COLUMN username TEXT;

-- Create unique index for username
CREATE UNIQUE INDEX idx_users_username ON users(username) WHERE username IS NOT NULL;
