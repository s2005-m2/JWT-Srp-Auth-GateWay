type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

interface ApiError {
  code: string;
  message: string;
}

interface ApiResponse<T> {
  data?: T;
  error?: ApiError;
}

const BASE_URL = (import.meta.env.BASE_URL || '/').replace(/\/$/, '');

let tokenGetter: (() => string | null) | null = null;
let onUnauthorized: (() => void) | null = null;

export function setApiConfig(config: {
  getToken: () => string | null;
  onUnauthorized: () => void;
}) {
  tokenGetter = config.getToken;
  onUnauthorized = config.onUnauthorized;
}

export async function api<T>(
  endpoint: string,
  method: HttpMethod = 'GET',
  body?: unknown
): Promise<ApiResponse<T>> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  const token = tokenGetter?.();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  try {
    const url = endpoint.startsWith('/') ? `${BASE_URL}${endpoint}` : endpoint;
    const res = await fetch(url, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (res.status === 401) {
      onUnauthorized?.();
      return { error: { code: 'UNAUTHORIZED', message: 'Session expired' } };
    }

    if (res.status === 429) {
      return { error: { code: 'RATE_LIMITED', message: 'Too many requests' } };
    }

    const data = await res.json();

    if (!res.ok) {
      return { error: data.error || { code: 'ERROR', message: 'Request failed' } };
    }

    return { data };
  } catch (err) {
    return { error: { code: 'NETWORK_ERROR', message: 'Network error' } };
  }
}

export const adminApi = {
  login: (username: string, password: string) =>
    api<{ admin: { id: string; username: string }; access_token: string }>(
      '/api/admin/login', 'POST', { username, password }
    ),

  register: (username: string, password: string, registration_token: string) =>
    api<{ admin: { id: string; username: string }; access_token: string }>(
      '/api/admin/register', 'POST', { username, password, registration_token }
    ),

  getStats: () => api<{ active_users: number; total_requests: number; system_status: string }>(
    '/api/admin/stats'
  ),

  getActivities: () => api<{ activities: Array<{ id: string; action: string; email: string; status: string; created_at: string }> }>(
    '/api/admin/activities'
  ),

  getUsers: () => api<{ users: Array<{ id: string; email: string; email_verified: boolean; created_at: string }> }>(
    '/api/admin/users'
  ),
};

export const configApi = {
  listRoutes: () => api<Array<{ id: string; path_prefix: string; upstream_address: string; require_auth: boolean; strip_prefix: string | null; enabled: boolean }>>(
    '/api/config/routes'
  ),

  createRoute: (data: { path_prefix: string; upstream_address: string; require_auth: boolean; strip_prefix?: string }) =>
    api<{ id: string; path_prefix: string; upstream_address: string; require_auth: boolean; strip_prefix: string | null; enabled: boolean }>('/api/config/routes', 'POST', data),

  updateRoute: (id: string, data: { path_prefix: string; upstream_address: string; require_auth: boolean; strip_prefix?: string; enabled: boolean }) =>
    api<{ id: string; path_prefix: string; upstream_address: string; require_auth: boolean; strip_prefix: string | null; enabled: boolean }>(`/api/config/routes/${id}`, 'PUT', data),

  deleteRoute: (id: string) => api(`/api/config/routes/${id}`, 'DELETE'),

  listRateLimits: () => api<Array<{ id: string; name: string; path_pattern: string; limit_by: string; max_requests: number; window_secs: number; enabled: boolean }>>(
    '/api/config/rate-limits'
  ),

  createRateLimit: (data: { name: string; path_pattern: string; limit_by: string; max_requests: number; window_secs: number }) =>
    api<{ id: string; name: string; path_pattern: string; limit_by: string; max_requests: number; window_secs: number; enabled: boolean }>('/api/config/rate-limits', 'POST', data),

  updateRateLimit: (id: string, data: { name: string; path_pattern: string; limit_by: string; max_requests: number; window_secs: number; enabled: boolean }) =>
    api<{ id: string; name: string; path_pattern: string; limit_by: string; max_requests: number; window_secs: number; enabled: boolean }>(`/api/config/rate-limits/${id}`, 'PUT', data),

  deleteRateLimit: (id: string) => api(`/api/config/rate-limits/${id}`, 'DELETE'),

  getJwtConfig: () => api<{ access_token_ttl_secs: number; refresh_token_ttl_secs: number; auto_refresh_threshold_secs: number }>(
    '/api/config/jwt'
  ),

  updateJwtConfig: (data: { access_token_ttl_secs: number; refresh_token_ttl_secs: number; auto_refresh_threshold_secs: number }) =>
    api('/api/config/jwt', 'PUT', data),

  getSmtpConfig: () => api<{ smtp_host: string; smtp_port: number; smtp_user: string; from_email: string; from_name: string }>(
    '/api/config/smtp'
  ),

  updateSmtpConfig: (data: { from_email: string; smtp_pass: string }) =>
    api('/api/config/smtp', 'PUT', data),

  getJwtSecretInfo: () => api<{ updated_at: string }>('/api/config/jwt-secret'),

  rotateJwtSecret: () => api('/api/config/jwt-secret', 'POST'),
};
