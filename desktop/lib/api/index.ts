// API client and endpoints

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "";

export interface ApiResponse<T> {
  data?: T;
  error?: string;
  status: number;
}

export async function fetchApi<T>(
  endpoint: string,
  options?: RequestInit
): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      ...options,
      headers: {
        "Content-Type": "application/json",
        ...options?.headers,
      },
    });

    const data = await response.json();

    return {
      data,
      status: response.status,
    };
  } catch (error) {
    return {
      error: error instanceof Error ? error.message : "Unknown error",
      status: 500,
    };
  }
}

// API endpoints
export const endpoints = {
  chat: {
    list: "/api/chat",
    create: "/api/chat",
    get: (id: string) => `/api/chat/${id}`,
    delete: (id: string) => `/api/chat/${id}`,
  },
  user: {
    profile: "/api/user/profile",
    update: "/api/user/profile",
  },
};
