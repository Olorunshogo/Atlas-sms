import { publicRequest } from '../../transport/public-request';

export interface PublicZone {
  id: number;
  name: string;
  distance_km: number;
  free_delivery: boolean;
  areas: string[];
}

export function fetchPublicZones(): Promise<PublicZone[]> {
  return publicRequest<PublicZone[]>('/zones');
}
