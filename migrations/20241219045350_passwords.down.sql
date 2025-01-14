-- Add down migration script here
ALTER TABLE users
DROP COLUMN hashed_password
