-- Add migration script here
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    encrypted_filename TEXT NOT NULL,
    encrypted_file_key TEXT NOT NULL,
    encryption_nonce TEXT NOT NULL,
    encryption_version TEXT NOT NULL DEFAULT '1',

    file_size BIGINT NOT NULL CHECK (file_size >= 0),

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_files_user_id ON files(user_id);