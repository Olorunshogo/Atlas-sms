import { publicRequest } from '../../transport/public-request';

export interface PublicTestimonial {
  score: number;
  comment: string;
  createdAt: string;
  authorLabel: string;
  avatarUrl: string | null;
  location: string;
}

export interface PublicTestimonialsResponse {
  items: PublicTestimonial[];
  summary: {
    score: string;
    count: number;
    displayable: boolean;
  };
}

export function fetchPublicTestimonials(): Promise<PublicTestimonialsResponse> {
  return publicRequest<PublicTestimonialsResponse>('/testimonials');
}
