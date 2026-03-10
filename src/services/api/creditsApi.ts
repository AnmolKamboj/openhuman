import type { ApiResponse } from '../../types/api';
import { apiClient } from '../apiClient';

interface CreditBalance {
  balance: number;
  currency: string;
}

interface CreditTransaction {
  id: string;
  amount: number;
  type: 'credit' | 'debit';
  description: string;
  createdAt: string;
  // Add other fields based on backend schema
}

interface PaginatedTransactions {
  transactions: CreditTransaction[];
  totalCount: number;
  page: number;
  limit: number;
}

/**
 * Credits API endpoints
 */
export const creditsApi = {
  /**
   * Get the current user's credit balance
   * GET /credits/balance
   */
  getBalance: async (): Promise<CreditBalance> => {
    const response = await apiClient.get<ApiResponse<CreditBalance>>('/credits/balance');
    return response.data;
  },

  /**
   * Get paginated credit transaction history
   * GET /credits/transactions
   */
  getTransactions: async (page = 1, limit = 50): Promise<PaginatedTransactions> => {
    const response = await apiClient.get<ApiResponse<PaginatedTransactions>>(
      `/credits/transactions?page=${page}&limit=${limit}`
    );
    return response.data;
  },
};
