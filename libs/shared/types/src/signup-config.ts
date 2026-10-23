import type { ZodType } from 'zod';

/*
 * From the reference source - a multi-role (buyer/agent/employee) signup flow with
 * per-role terms, referral codes, and CV upload. Atlas's actual signup is a
 * single invite-based flow (see docs/atlas/auth-flow.md) - this is a prune
 * candidate, kept only as reference for the per-role-config shape.
 *
 * The original imported AuthFieldDescriptor from a components/auth/auth-field
 * module that was never part of this reference tree (dangling even in the
 * source) - defined locally below as a minimal placeholder so this file
 * type-checks; replace with Atlas's own field-descriptor type if this pattern
 * is ever revived.
 *
 * SignupValues also originally came from @atlas/validation's auth/signup -
 * inlined locally instead (just the base fields) to avoid a circular
 * workspace dependency (validation already depends on data, which depends
 * on types; types depending back on validation would complete a cycle).
 */

interface SignupValues {
  fullName: string;
  email: string;
  phone: string;
  password: string;
  confirmPassword: string;
}

/** Placeholder for the original AuthFieldDescriptor - see file comment. */
export interface AuthFieldDescriptor {
  name: string;
  label: string;
  type: string;
}

export interface SignupFormValues extends SignupValues {
  referredByAgentCode?: string;
  lga?: string;
  address?: string;
  cv?: File;
  acceptedTerms?: boolean;
}

/** Name split off the single full-name input, plus the role being created. */
export interface SignupIdentity {
  first_name: string;
  last_name: string;
  role: string;
}

/** The document a role consents to at signup, and the version it was on. */
export interface RoleTermsRef {
  slug: string;
  version: string;
}

export interface RoleSignupConfig {
  role: string;
  label: string;
  schema: ZodType<SignupFormValues, SignupFormValues>;
  fields: readonly AuthFieldDescriptor[];
  terms?: RoleTermsRef;
  redirectTo: string;
  successTitle: string;
  successDescription: string;
  register?: (
    values: SignupFormValues,
    identity: SignupIdentity,
  ) => Promise<void>;
}
