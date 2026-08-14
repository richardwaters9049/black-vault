import { API_URL } from "../config";
import {
  User,
  AuthResponse,
  RegisterRequest,
  LoginRequest,
  ApiError as ApiErrorType,
} from "./types";

class ApiError extends Error {
  constructor(
    public message: string,
    public status?: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const errorData: ApiErrorType = await response.json();
    throw new ApiError(errorData.error, response.status);
  }
  return response.json();
}

export const api = {
  async register(data: RegisterRequest): Promise<User> {
    const response = await fetch(`${API_URL}/auth/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return handleResponse<User>(response);
  },

  async login(data: LoginRequest): Promise<AuthResponse> {
    const response = await fetch(`${API_URL}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return handleResponse<AuthResponse>(response);
  },

  async getCurrentUser(token: string): Promise<User> {
    const response = await fetch(`${API_URL}/user/me`, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    return handleResponse<User>(response);
  },

  async healthCheck(): Promise<string> {
    const response = await fetch(`${API_URL}/health`);
    return response.text();
  },
};
