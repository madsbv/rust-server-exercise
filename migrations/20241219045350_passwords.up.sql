-- Add up migration script here
ALTER TABLE users
ADD COLUMN hashed_password TEXT NOT NULL DEFAULT 'unset'
