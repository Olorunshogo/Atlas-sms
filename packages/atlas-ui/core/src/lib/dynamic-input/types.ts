export interface PhoneCountry {
  value: string;
  label: string;
  code: string;
  flagImage?: string;
  phoneLength: number;
}

export interface DynamicInputOption {
  value: string;
  label: string;
  disabled?: boolean;
}
