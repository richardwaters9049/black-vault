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
