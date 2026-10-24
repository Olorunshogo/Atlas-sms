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
    /// A request the framework refused before a handler ran (bad JSON, bad path, unknown route); keeps the status axum chose.
    Rejected {
        status: StatusCode,
        message: String,
        code: &'static str,
    },
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

    pub fn rejected(status: StatusCode, message: String, code: &'static str) -> Self {
        return ApiError::Rejected { status, message, code };
    }
}

// === Error codes

/// Machine-readable `code` values for the sales, payments, returns and customer flows, kept here so the POS can switch on one list instead of scattered literals.
pub mod codes {
    pub const EMAIL_NOT_VERIFIED: &str = "EMAIL_NOT_VERIFIED";
    pub const PRODUCT_OUT_OF_STOCK: &str = "PRODUCT_OUT_OF_STOCK";
    pub const PRODUCT_NOT_SOLD_HERE: &str = "PRODUCT_NOT_SOLD_HERE";
    pub const PRODUCT_NOT_PRICED: &str = "PRODUCT_NOT_PRICED";
    pub const INVALID_UNIT: &str = "INVALID_UNIT";
    pub const STOCK_ITEM_REQUIRED: &str = "STOCK_ITEM_REQUIRED";
    pub const PRICE_OVERRIDE_FORBIDDEN: &str = "PRICE_OVERRIDE_FORBIDDEN";
    pub const DISCOUNT_FORBIDDEN: &str = "DISCOUNT_FORBIDDEN";
    pub const SHIFT_NOT_OPEN: &str = "SHIFT_NOT_OPEN";
    pub const STORE_INACTIVE: &str = "STORE_INACTIVE";
    pub const PRODUCT_INACTIVE: &str = "PRODUCT_INACTIVE";
    pub const IDEMPOTENCY_CONFLICT: &str = "IDEMPOTENCY_CONFLICT";
    pub const TOTAL_MISMATCH: &str = "TOTAL_MISMATCH";
    pub const PAYMENT_INSUFFICIENT: &str = "PAYMENT_INSUFFICIENT";
    pub const PAYMENT_OVERPAID: &str = "PAYMENT_OVERPAID";
    pub const PAYMENT_REFERENCE_REQUIRED: &str = "PAYMENT_REFERENCE_REQUIRED";
    pub const PAYMENT_METHOD_INVALID: &str = "PAYMENT_METHOD_INVALID";
    pub const DUPLICATE_PAYMENT_REFERENCE: &str = "DUPLICATE_PAYMENT_REFERENCE";
    pub const INVALID_PAYMENT_TRANSITION: &str = "INVALID_PAYMENT_TRANSITION";
    pub const RETURN_QUANTITY_EXCEEDED: &str = "RETURN_QUANTITY_EXCEEDED";
    pub const REFUND_AMOUNT_EXCEEDED: &str = "REFUND_AMOUNT_EXCEEDED";
    pub const ITEM_NOT_RETURNABLE: &str = "ITEM_NOT_RETURNABLE";
    pub const APPROVAL_SELF_FORBIDDEN: &str = "APPROVAL_SELF_FORBIDDEN";
    pub const APPROVAL_REQUIRED: &str = "APPROVAL_REQUIRED";
    pub const SALE_NOT_COMPLETED: &str = "SALE_NOT_COMPLETED";
    pub const SALE_NOT_VOIDABLE: &str = "SALE_NOT_VOIDABLE";
    pub const RETURN_NOT_PENDING: &str = "RETURN_NOT_PENDING";
    pub const CUSTOMER_REQUIRED: &str = "CUSTOMER_REQUIRED";
    pub const CUSTOMER_PHONE_EXISTS: &str = "CUSTOMER_PHONE_EXISTS";
    pub const BARCODE_EXISTS: &str = "BARCODE_EXISTS";
    pub const PAYMENT_METHOD_EXISTS: &str = "PAYMENT_METHOD_EXISTS";
    pub const SHIFT_ALREADY_OPEN: &str = "SHIFT_ALREADY_OPEN";
    pub const SHIFT_OPEN: &str = "SHIFT_OPEN";
    pub const TRANSFER_NOT_REQUESTED: &str = "TRANSFER_NOT_REQUESTED";
    pub const TRANSFER_NOT_APPROVED: &str = "TRANSFER_NOT_APPROVED";
    pub const TRANSFER_NOT_DISPATCHED: &str = "TRANSFER_NOT_DISPATCHED";
    pub const TRANSFER_NOT_CANCELLABLE: &str = "TRANSFER_NOT_CANCELLABLE";
    pub const STOCK_COUNT_NOT_PENDING: &str = "STOCK_COUNT_NOT_PENDING";
    pub const BATCH_REQUIRED: &str = "BATCH_REQUIRED";
    pub const BATCH_EXPIRED: &str = "BATCH_EXPIRED";
    pub const PURCHASE_ORDER_NOT_DRAFT: &str = "PURCHASE_ORDER_NOT_DRAFT";
    pub const PURCHASE_ORDER_NOT_CANCELLABLE: &str = "PURCHASE_ORDER_NOT_CANCELLABLE";
    pub const PURCHASE_ORDER_NOT_SUBMITTED: &str = "PURCHASE_ORDER_NOT_SUBMITTED";
    pub const PURCHASE_ORDER_NOT_RECEIVABLE: &str = "PURCHASE_ORDER_NOT_RECEIVABLE";
    pub const RECEIPT_EXCEEDS_OUTSTANDING: &str = "RECEIPT_EXCEEDS_OUTSTANDING";
    pub const PURCHASE_RETURN_NOT_PENDING: &str = "PURCHASE_RETURN_NOT_PENDING";
    pub const SUPPLIER_INACTIVE: &str = "SUPPLIER_INACTIVE";
    pub const EXPENSE_NOT_DRAFT: &str = "EXPENSE_NOT_DRAFT";
    pub const EXPENSE_NOT_SUBMITTED: &str = "EXPENSE_NOT_SUBMITTED";
    pub const EXPENSE_NOT_APPROVED: &str = "EXPENSE_NOT_APPROVED";
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
            ApiError::Rejected { status, message, code } => (status, message, Some(code)),
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

/**
 * Constraint violations are the caller's mistake, not a server fault, so they become 4xx instead of a blanket 500.
 * The message is generic on purpose: constraint and column names are database internals.
 */
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err {
            match db_err.code().as_deref() {
                // unique_violation
                Some("23505") => {
                    return ApiError::conflict_with_code("A record with these details already exists.", "ALREADY_EXISTS");
                }
                // foreign_key_violation
                Some("23503") => {
                    return ApiError::bad_request_with_code("A referenced record does not exist.", "INVALID_REFERENCE");
                }
                // check_violation
                Some("23514") => {
                    return ApiError::bad_request_with_code("A value is out of the allowed range.", "INVALID_VALUE");
                }
                _ => {}
            }
        }
        return ApiError::Internal(err.to_string());
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(err: argon2::password_hash::Error) -> Self {
        return ApiError::Internal(format!("password hashing failure: {}", err));
    }
}
