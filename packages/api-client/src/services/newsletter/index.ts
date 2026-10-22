import { publicPost } from '../../transport/public-request';

export function subscribeToNewsletter(email: string): Promise<null> {
  return publicPost<null>('/newsletter', { email });
}
