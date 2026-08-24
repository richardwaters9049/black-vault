// API Types matching the backend contract

export interface User {
  id: string;
  email: string;
}

export interface AuthResponse {
  id: string;
  email: string;
  token: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface ApiError {
  error: string;
}

export interface DerivationParams {
  salt: string;
  t_cost: number;
  m_cost: number;
  p_cost: number;
  output_len: number;
}

export interface SetupVaultRequest {
  encrypted_vault_key: string;
  key_derivation_salt: string;
  key_derivation_params: DerivationParams;
  encryption_nonce: string;
}

export interface SetupVaultResponse {
  success: boolean;
}

export interface GetVaultKeyResponse {
  encrypted_vault_key: string;
  key_derivation_salt: string;
  key_derivation_params: DerivationParams;
  encryption_nonce: string;
  encryption_version: string;
}
