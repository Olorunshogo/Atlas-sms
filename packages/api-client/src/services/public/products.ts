import { publicRequest } from '../../transport/public-request';

export interface PublicProduct {
  id: number;
  name: string;
  slug: string;
  unit: string;
  price_kobo: number;
  description: string | null;
  image_url: string | null;
  images?: string[];
  category: string | null;
  subcategory?: string | null;
  category_id?: number | null;
  measure_value?: number | null;
  measure_unit?: string | null;
  weight_grams?: number | null;
  sort_order?: number;
}

export function fetchPublicProducts(): Promise<PublicProduct[]> {
  return publicRequest<PublicProduct[]>('/products');
}

export function fetchPublicProduct(
  slugOrId: string | number,
): Promise<PublicProduct> {
  return publicRequest<PublicProduct>(`/products/${slugOrId}`);
}
