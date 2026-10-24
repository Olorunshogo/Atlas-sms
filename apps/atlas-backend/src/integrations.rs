/*!
 * Third parties the storefront talks to, as adapters: a payment gateway, a courier and social sign-in.
 * Handlers only see these traits on `AppState`; a real Paystack, Flutterwave, courier or Google adapter is one new
 * implementation plus a line in `build_state`. The mocks verify webhooks with a shared-secret SHA-256 signature, the
 * same shape a real provider's HMAC check takes.
 */
use std::sync::Arc;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct IntegrationError(pub String);

impl std::fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.0);
    }
}

/// Hex SHA-256 of `secret || body`: what the mock providers send as their webhook signature.
pub fn mock_signature(secret: &[u8], body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(body);
    return hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
}

fn check_signature(secret: &[u8], signature: Option<&str>, body: &[u8]) -> Result<(), IntegrationError> {
    let expected = mock_signature(secret, body);
    match signature {
        Some(given) if given.eq_ignore_ascii_case(&expected) => Ok(()),
        _ => Err(IntegrationError("webhook signature does not match".to_string())),
    }
}

fn short(id: Uuid) -> String {
    return id.simple().to_string()[20..].to_string();
}

// === Payments

#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub method: String,
    pub amount_minor: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct PaymentInit {
    pub gateway_ref: String,
    /// Where to send the customer for card and wallet payments.
    pub redirect_url: Option<String>,
    /// What to show for bank transfer and USSD, which the customer completes elsewhere.
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentState {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PaymentEvent {
    pub gateway_ref: String,
    pub state: PaymentState,
    pub reason: Option<String>,
}

#[async_trait::async_trait]
pub trait PaymentGateway: Send + Sync {
    fn name(&self) -> &'static str;
    async fn initiate(&self, request: &PaymentRequest) -> Result<PaymentInit, IntegrationError>;
    /// The server-side check for an attempt that never called back.
    async fn verify(&self, gateway_ref: &str) -> Result<PaymentState, IntegrationError>;
    async fn refund(&self, gateway_ref: &str, amount_minor: i64) -> Result<(), IntegrationError>;
    /// The only way a payment counts as successful: a signed callback from the gateway itself.
    fn parse_webhook(&self, signature: Option<&str>, body: &[u8]) -> Result<PaymentEvent, IntegrationError>;
}

pub struct MockPaymentGateway {
    secret: Vec<u8>,
}

#[derive(Deserialize)]
struct MockPaymentWebhook {
    gateway_ref: String,
    status: String,
    reason: Option<String>,
}

#[async_trait::async_trait]
impl PaymentGateway for MockPaymentGateway {
    fn name(&self) -> &'static str {
        return "mock";
    }

    async fn initiate(&self, request: &PaymentRequest) -> Result<PaymentInit, IntegrationError> {
        let gateway_ref = format!("mockpay_{}", Uuid::now_v7().simple());
        let (redirect_url, instructions) = match request.method.as_str() {
            "card" | "mobile_wallet" => (Some(format!("https://mock-gateway.local/pay/{}", gateway_ref)), None),
            "bank_transfer" => (None, Some(format!("Transfer {} to 0123456789 (Mock Bank), reference {}", request.amount_minor, gateway_ref))),
            "ussd" => (None, Some(format!("Dial *000*{}#", &gateway_ref[gateway_ref.len() - 8..]))),
            other => return Err(IntegrationError(format!("the mock gateway doesn't take {}", other))),
        };
        return Ok(PaymentInit { gateway_ref, redirect_url, instructions });
    }

    async fn verify(&self, _gateway_ref: &str) -> Result<PaymentState, IntegrationError> {
        return Ok(PaymentState::Pending);
    }

    async fn refund(&self, _gateway_ref: &str, _amount_minor: i64) -> Result<(), IntegrationError> {
        return Ok(());
    }

    fn parse_webhook(&self, signature: Option<&str>, body: &[u8]) -> Result<PaymentEvent, IntegrationError> {
        check_signature(&self.secret, signature, body)?;
        let event: MockPaymentWebhook = serde_json::from_slice(body).map_err(|e| IntegrationError(e.to_string()))?;
        let state = match event.status.as_str() {
            "succeeded" => PaymentState::Succeeded,
            "failed" => PaymentState::Failed,
            other => return Err(IntegrationError(format!("unknown payment status {}", other))),
        };
        return Ok(PaymentEvent { gateway_ref: event.gateway_ref, state, reason: event.reason });
    }
}

// === Courier

#[derive(Debug, Clone)]
pub struct ShipmentRequest {
    pub order_number: String,
    pub address: serde_json::Value,
    pub weight_kg: rust_decimal::Decimal,
}

#[derive(Debug, Clone)]
pub struct ShipmentBooking {
    pub tracking_number: String,
    pub label_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipmentState {
    InTransit,
    Delivered,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ShipmentEvent {
    pub tracking_number: String,
    pub state: ShipmentState,
    pub note: Option<String>,
}

#[async_trait::async_trait]
pub trait CourierProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn create_shipment(&self, request: &ShipmentRequest) -> Result<ShipmentBooking, IntegrationError>;
    fn parse_webhook(&self, signature: Option<&str>, body: &[u8]) -> Result<ShipmentEvent, IntegrationError>;
}

pub struct MockCourier {
    secret: Vec<u8>,
}

#[derive(Deserialize)]
struct MockCourierWebhook {
    tracking_number: String,
    status: String,
    note: Option<String>,
}

#[async_trait::async_trait]
impl CourierProvider for MockCourier {
    fn name(&self) -> &'static str {
        return "mock";
    }

    async fn create_shipment(&self, request: &ShipmentRequest) -> Result<ShipmentBooking, IntegrationError> {
        let tracking_number = format!("MOCKTRK{}", short(Uuid::now_v7()).to_uppercase());
        let _ = (&request.order_number, &request.address, request.weight_kg);
        return Ok(ShipmentBooking {
            label_url: format!("https://mock-courier.local/labels/{}.pdf", tracking_number),
            tracking_number,
        });
    }

    fn parse_webhook(&self, signature: Option<&str>, body: &[u8]) -> Result<ShipmentEvent, IntegrationError> {
        check_signature(&self.secret, signature, body)?;
        let event: MockCourierWebhook = serde_json::from_slice(body).map_err(|e| IntegrationError(e.to_string()))?;
        let state = match event.status.as_str() {
            "in_transit" => ShipmentState::InTransit,
            "delivered" => ShipmentState::Delivered,
            "failed" => ShipmentState::Failed,
            other => return Err(IntegrationError(format!("unknown shipment status {}", other))),
        };
        return Ok(ShipmentEvent { tracking_number: event.tracking_number, state, note: event.note });
    }
}

// === Social sign-in

#[derive(Debug, Clone)]
pub struct SocialIdentity {
    pub subject: String,
    pub email: String,
    pub full_name: String,
}

#[async_trait::async_trait]
pub trait SocialIdentityProvider: Send + Sync {
    /// Checks a provider's sign-in token server-side and returns who it belongs to.
    async fn verify(&self, provider: &str, token: &str) -> Result<SocialIdentity, IntegrationError>;
}

/// Accepts `mock:{subject}:{email}:{full name}` for `google` or `facebook`, and nothing else.
pub struct MockSocialIdentity;

#[async_trait::async_trait]
impl SocialIdentityProvider for MockSocialIdentity {
    async fn verify(&self, provider: &str, token: &str) -> Result<SocialIdentity, IntegrationError> {
        if provider != "google" && provider != "facebook" {
            return Err(IntegrationError(format!("unsupported provider {}", provider)));
        }
        let parts: Vec<&str> = token.splitn(4, ':').collect();
        match parts.as_slice() {
            ["mock", subject, email, name] if !subject.is_empty() && email.contains('@') && !name.is_empty() => Ok(SocialIdentity {
                subject: subject.to_string(),
                email: email.to_string(),
                full_name: name.to_string(),
            }),
            _ => Err(IntegrationError("the sign-in token is not valid".to_string())),
        }
    }
}

// === Bundle

#[derive(Clone)]
pub struct Integrations {
    pub payments: Arc<dyn PaymentGateway>,
    pub courier: Arc<dyn CourierProvider>,
    pub social: Arc<dyn SocialIdentityProvider>,
}

impl Integrations {
    /// Every provider mocked, webhooks signed with `secret`.
    pub fn mock(secret: &[u8]) -> Integrations {
        return Integrations {
            payments: Arc::new(MockPaymentGateway { secret: secret.to_vec() }),
            courier: Arc::new(MockCourier { secret: secret.to_vec() }),
            social: Arc::new(MockSocialIdentity),
        };
    }
}
