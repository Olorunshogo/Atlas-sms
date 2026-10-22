use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

// === ApiError

/// Every handler error path funnels through this type so the response shape (`{ message, success, code }`) stays consistent across the whole API.
#[derive(Debug)]
pub enum ApiError {
    BadRequest {
        message: String,
        code: Option<&'static str>,
    },
    Unauthorized(String),
    Forbidden {
        message: String,
        code: Option<&'static str>,
    },
    NotFound(String),
    Conflict {
        message: String,
        code: Option<&'static str>,
    },
    TooManyRequests(String),
    Internal(String),
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        return ApiError::BadRequest {
            message: message.into(),
            code: None,
        };
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        return ApiError::Forbidden {
            message: message.into(),
            code: None,
        };
    }

    pub fn bad_request_with_code(message: impl Into<String>, code: &'static str) -> Self {
        return ApiError::BadRequest {
            message: message.into(),
            code: Some(code),
        };
    }

    pub fn conflict_with_code(message: impl Into<String>, code: &'static str) -> Self {
        return ApiError::Conflict {
            message: message.into(),
            code: Some(code),
        };
    }

    pub fn forbidden_with_code(message: impl Into<String>, code: &'static str) -> Self {
        return ApiError::Forbidden {
            message: message.into(),
            code: Some(code),
        };
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        return ApiError::NotFound(message.into());
    }
}

// === Error codes

/// Machine-readable `code` values for the sales, payments, returns and customer flows, kept here so the POS can switch on one list instead of scattered literals.
#[allow(dead_code)]
pub mod codes {
    pub const PRODUCT_OUT_OF_STOCK: &str = "PRODUCT_OUT_OF_STOCK";
    pub const PRODUCT_NOT_SOLD_HERE: &str = "PRODUCT_NOT_SOLD_HERE";
    pub const PRODUCT_NOT_PRICED: &str = "PRODUCT_NOT_PRICED";
    pub const INVALID_UNIT: &str = "INVALID_UNIT";
    pub const STOCK_ITEM_REQUIRED: &str = "STOCK_ITEM_REQUIRED";
    pub const PRICE_OVERRIDE_FORBIDDEN: &str = "PRICE_OVERRIDE_FORBIDDEN";
    pub const DISCOUNT_FORBIDDEN: &str = "DISCOUNT_FORBIDDEN";
    pub const SHIFT_NOT_OPEN: &str = "SHIFT_NOT_OPEN";
    pub const STORE_INACTIVE: &str = "STORE_INACTIVE";
    pub const IDEMPOTENCY_CONFLICT: &str = "IDEMPOTENCY_CONFLICT";
    pub const TOTAL_MISMATCH: &str = "TOTAL_MISMATCH";
    pub const PAYMENT_INSUFFICIENT: &str = "PAYMENT_INSUFFICIENT";
    pub const PAYMENT_OVERPAID: &str = "PAYMENT_OVERPAID";
    pub const PAYMENT_REFERENCE_REQUIRED: &str = "PAYMENT_REFERENCE_REQUIRED";
    pub const PAYMENT_METHOD_INVALID: &str = "PAYMENT_METHOD_INVALID";
    pub const DUPLICATE_PAYMENT_REFERENCE: &str = "DUPLICATE_PAYMENT_REFERENCE";
    pub const INVALID_PAYMENT_TRANSITION: &str = "INVALID_PAYMENT_TRANSITION";
    pub const RETURN_WINDOW_EXPIRED: &str = "RETURN_WINDOW_EXPIRED";
    pub const RETURN_QUANTITY_EXCEEDED: &str = "RETURN_QUANTITY_EXCEEDED";
    pub const REFUND_AMOUNT_EXCEEDED: &str = "REFUND_AMOUNT_EXCEEDED";
    pub const ITEM_NOT_RETURNABLE: &str = "ITEM_NOT_RETURNABLE";
    pub const APPROVAL_REQUIRED: &str = "APPROVAL_REQUIRED";
    pub const APPROVAL_SELF_FORBIDDEN: &str = "APPROVAL_SELF_FORBIDDEN";
    pub const SALE_NOT_COMPLETED: &str = "SALE_NOT_COMPLETED";
    pub const SALE_NOT_VOIDABLE: &str = "SALE_NOT_VOIDABLE";
    pub const RETURN_NOT_PENDING: &str = "RETURN_NOT_PENDING";
    pub const CUSTOMER_REQUIRED: &str = "CUSTOMER_REQUIRED";
    pub const CUSTOMER_PHONE_EXISTS: &str = "CUSTOMER_PHONE_EXISTS";
    pub const STORE_REQUIRED: &str = "STORE_REQUIRED";
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message, code) = match self {
            ApiError::BadRequest { message, code } => (StatusCode::BAD_REQUEST, message, code),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message, None),
            ApiError::Forbidden { message, code } => (StatusCode::FORBIDDEN, message, code),
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message, None),
            ApiError::Conflict { message, code } => (StatusCode::CONFLICT, message, code),
            ApiError::TooManyRequests(message) => (StatusCode::TOO_MANY_REQUESTS, message, None),
            ApiError::Internal(message) => {
                eprintln!("internal error: {}", message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_string(),
                    None,
                )
            }
        };

        return (
            status,
            Json(ErrorBody {
                message,
                success: false,
                code,
            }),
        )
            .into_response();
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        return ApiError::Internal(err.to_string());
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(err: argon2::password_hash::Error) -> Self {
        return ApiError::Internal(format!("password hashing failure: {}", err));
    }
}
