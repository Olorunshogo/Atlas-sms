use serde::{Deserialize, Serialize};
use uuid::Uuid;

// === Signup

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub password: String,
    /// Names the tenant created alongside the user, who becomes its `owner`.
    pub tenant_name: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResponseData {
    pub email: String,
}

// === Verify email

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct ResendVerificationRequest {
    pub email: String,
}

// === Login / session

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    /// Only required when the user holds active grants in more than one tenant.
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    /// `false` keeps every tenant-scoped endpoint closed (`403 EMAIL_NOT_VERIFIED`) until the user verifies.
    pub email_verified: bool,
}

/**
 * `access_token`/`refresh_token` are `None` for `X-Client-Type: web`: the
 * Nitro proxy is the only thing that ever sees them, as `Set-Cookie`
 * headers, never in this JSON body - a web SSR payload or an error
 * reporter capturing the response body would otherwise leak them. Present
 * only for `X-Client-Type: app` (`atlas-pos`), which has no server of its
 * own to hold an HttpOnly cookie for it. See
 * `docs/atlas/auth-flow.md` #7.
 */
#[derive(Debug, Serialize)]
pub struct SessionData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub tenant_id: Uuid,
    pub user: AuthenticatedUser,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    /**
     * Required for `X-Client-Type: app`, which has no cookie.
     * Web sends an empty body. The handler reads the refresh cookie instead.
     */
    #[serde(default)]
    pub refresh_token: String,
}

// === Tenant choice

/**
 * A tenant the caller can log into or switch to.
 * Returned with `TENANT_REQUIRED` and `PLATFORM_TENANT_REQUIRED`, and on `GET /auth/me`.
 */
#[derive(Debug, Serialize)]
pub struct TenantChoice {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TenantChoiceList {
    pub tenants: Vec<TenantChoice>,
}

#[derive(Debug, Serialize)]
pub struct ChoiceErrorBody {
    pub message: String,
    pub success: bool,
    pub code: &'static str,
    pub data: TenantChoiceList,
}

#[derive(Debug, Serialize)]
pub struct SessionProfile {
    pub user: AuthenticatedUser,
    pub tenant_id: Uuid,
    pub is_platform_admin: bool,
    pub tenants: Vec<TenantChoice>,
}

#[derive(Debug, Deserialize)]
pub struct PlatformInviteRequest {
    pub email: String,
}

// === Switch tenant

#[derive(Debug, Deserialize)]
pub struct SwitchTenantRequest {
    pub tenant_id: Uuid,
}

// === Accept invite

#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    /// Only used when no account exists yet for the invite's email -
    /// ignored (the existing name/password stand) if one already does.
    pub full_name: String,
    pub phone: String,
    pub password: String,
}

// === Forgot / reset password

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub code: String,
    pub new_password: String,
}
