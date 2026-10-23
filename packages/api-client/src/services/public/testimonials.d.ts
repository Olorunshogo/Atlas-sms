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
export declare function fetchPublicTestimonials(): Promise<PublicTestimonialsResponse>;
