// === Location types

/* Shared shapes for Nigerian administrative geography. Consumed by the
   nigerian-states dataset and by any form that needs a state, LGA, or area
   selector. */

/* areas is named areas/wards within the LGA. Only populated where we have the detail. */
export interface NigerianLga {
  name: string;
  areas?: readonly string[];
}

/* slug is kebab-case, URL/param safe, derived from name. */
export interface NigerianState {
  name: string;
  capital: string;
  slug: string;
  lgas: readonly NigerianLga[];
}

/* Mirrors the local `Option` shape in components/select-field.tsx so select
   options built from this data drop straight into the existing inputs. */
export interface SelectOption {
  value: string;
  label: string;
}
