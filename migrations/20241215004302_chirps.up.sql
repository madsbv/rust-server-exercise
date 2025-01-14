-- Add up migration script here
CREATE TABLE chirps (
chirp_id UUID PRIMARY KEY,
user_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,
created_at TIMESTAMP WITH TIME ZONE,
updated_at TIMESTAMP WITH TIME ZONE,
body TEXT NOT NULL
)
