CREATE TABLE vault_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    encrypted_vault_key TEXT NOT NULL,
    key_derivation_salt TEXT NOT NULL,
    key_derivation_params JSONB NOT NULL,
    encryption_nonce TEXT NOT NULL,
    encryption_version TEXT NOT NULL DEFAULT '1',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_vault_keys_user_id ON vault_keys(user_id);
