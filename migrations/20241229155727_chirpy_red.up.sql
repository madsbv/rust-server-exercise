-- Add up migration script here
ALTER TABLE users
ADD COLUMN is_chirpy_red BOOLEAN NOT NULL DEFAULT false
