// === Location types

/* Shared shapes for Nigerian administrative geography. Consumed by the
   nigerian-states dataset (@atlas/data) and by any form that needs a state,
   LGA, or area selector. Generic, applies directly to Atlas. */

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

export interface SelectOption {
  value: string;
  label: string;
}
