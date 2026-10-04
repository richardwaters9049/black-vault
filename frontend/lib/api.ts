import type { FileRecord, FileUpload } from './file-crypto';
import { API_URL } from "../config";
import {
  User,
  AuthResponse,
  RegisterRequest,
  LoginRequest,
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

async function request(...args: Parameters<typeof fetch>): Promise<Response> {
  try { return await fetch(...args); }
  catch (error) {
    throw new ApiError(error instanceof Error && error.name === 'AbortError'
      ? 'Request cancelled.' : 'Could not reach the server. Please try again.');
  }
}

async function handleResponse<T>(response: Response): Promise<T> {
  const contentType = response.headers.get("content-type");
  const isJson = contentType?.includes("application/json") ?? false;

  if (!response.ok) {
    let message = 'Request failed';
    if (isJson) {
      // Preserve the HTTP failure even when a proxy returns malformed JSON.
      try {
        const data: unknown = await response.json();
        if (typeof data === 'object' && data !== null && 'error' in data
          && typeof data.error === 'string' && data.error.trim()) {
          message = data.error.slice(0, 512);
        }
      } catch { /* Fall back to the bounded HTTP error. */ }
    }
    throw new ApiError(message, response.status);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  if (!isJson) {
    return response.text() as Promise<T>;
  }

  try { return await response.json(); }
  catch { throw new ApiError('The server returned an invalid JSON response.', response.status); }
}

export const api = {
  async uploadFile(token: string, data: FileUpload): Promise<FileRecord> {
    return handleResponse<FileRecord>(await request(`${API_URL}/files/encrypted`, {
      method: 'POST', headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    }));
  },
  async listFiles(token: string): Promise<{ files: FileRecord[] }> {
    return handleResponse(await request(`${API_URL}/files`, { headers: { Authorization: `Bearer ${token}` } }));
  },
  async downloadFile(token: string, id: string): Promise<Uint8Array> {
    const response = await request(`${API_URL}/files/${encodeURIComponent(id)}/data`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!response.ok) await handleResponse(response);
    return new Uint8Array(await response.arrayBuffer());
  },
  async deleteFile(token: string, id: string): Promise<void> {
    return handleResponse(await request(`${API_URL}/files/${encodeURIComponent(id)}`, {
      method: 'DELETE', headers: { Authorization: `Bearer ${token}` },
    }));
  },
  async register(data: RegisterRequest): Promise<User> {
    const response = await request(`${API_URL}/auth/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return handleResponse<User>(response);
  },

  async login(data: LoginRequest): Promise<AuthResponse> {
    const response = await request(`${API_URL}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    return handleResponse<AuthResponse>(response);
  },

  async getCurrentUser(token: string): Promise<User> {
    const response = await request(`${API_URL}/user/me`, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    return handleResponse<User>(response);
  },

  async healthCheck(): Promise<string> {
    const response = await request(`${API_URL}/health`);
    return handleResponse<string>(response);
  },

  async setupVault(
    token: string,
    data: SetupVaultRequest,
  ): Promise<SetupVaultResponse> {
    const response = await request(`${API_URL}/vault/setup`, {
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
    const response = await request(`${API_URL}/vault/key`, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    return handleResponse<GetVaultKeyResponse>(response);
  },
};
