import { publicRequest } from '../../transport/public-request';

export function fetchOutreachShopCount(): Promise<{ count: number }> {
  return publicRequest<{ count: number }>('/outreach/count');
}
