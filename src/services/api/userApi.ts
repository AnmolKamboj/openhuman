import { apiClient } from '../apiClient';
import type { GetMeResponse, User } from '../../types/api';

/**
 * User API endpoints
 */
export const userApi = {
  /**
   * Get current authenticated user information
   * GET /telegram/me
   */
  getMe: async (): Promise<User> => {
    const response = await apiClient.get<GetMeResponse>('/telegram/me');
    if (!response.success) {
      throw new Error(response.error || 'Failed to fetch user data');
    }
    return response.data;
  },
};
