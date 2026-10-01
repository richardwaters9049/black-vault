import type { FileRecord, FileUpload } from './file-crypto';
import { API_URL } from "../config";
import {
  User,
  AuthResponse,
  RegisterRequest,
  LoginRequest,
  ApiError as ApiErrorType,
  SetupVaultRequest,
  SetupVaultResponse,
  GetVaultKeyResponse,
} from "./types";

export class ApiError extends Error {
  constructor(
    public message: string,
    public status?: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function handleResponse<T>(response: Response): Promise<T> {
  const contentType = response.headers.get("content-type");
  const isJson = contentType?.includes("application/json") ?? false;

  if (!response.ok) {
    if (isJson) {
      const errorData = (await response.json()) as Partial<ApiErrorType>;
      throw new ApiError(
        errorData.error ?? "Request failed",
        response.status,
      );
    }

    throw new ApiError("Request failed", response.status);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  if (!isJson) {
    return response.text() as Promise<T>;
  }

  return response.json();
}

export const api = {
  async uploadFile(token: string, data: FileUpload): Promise<FileRecord> {
    return handleResponse<FileRecord>(await fetch(`${API_URL}/files/encrypted`, {
      method: 'POST', headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }));
  },
  async listFiles(token: string): Promise<{ files: FileRecord[] }> {
    return handleResponse(await fetch(`${API_URL}/files`, { headers: { Authorization: `Bearer ${token}` } }));
  },
  async downloadFile(token: string, id: string): Promise<Uint8Array> {
    const response = await fetch(`${API_URL}/files/${encodeURIComponent(id)}/data`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!response.ok) await handleResponse(response);
    return new Uint8Array(await response.arrayBuffer());
  },
  async deleteFile(token: string, id: string): Promise<void> {
    return handleResponse(await fetch(`${API_URL}/files/${encodeURIComponent(id)}`, {
      method: 'DELETE', headers: { Authorization: `Bearer ${token}` },
    }));
  },
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
    return handleResponse<string>(response);
  },

  async setupVault(
    token: string,
    data: SetupVaultRequest,
  ): Promise<SetupVaultResponse> {
    const response = await fetch(`${API_URL}/vault/setup`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(data),
    });
    return handleResponse<SetupVaultResponse>(response);
  },

  async getVaultKey(token: string): Promise<GetVaultKeyResponse> {
    const response = await fetch(`${API_URL}/vault/key`, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    return handleResponse<GetVaultKeyResponse>(response);
  },
};
