-- Add up migration script here
CREATE TABLE users (
       id UUID PRIMARY KEY,
       created_at TIMESTAMP WITH TIME ZONE,
       updated_at TIMESTAMP WITH TIME ZONE,
       email TEXT UNIQUE NOT NULL
)

