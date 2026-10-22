use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

/// Mirrors `atlas.stock_movement_type`. The direction of a movement comes from its type, so callers pass magnitudes and can't get a sign wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementType {
    PurchaseReceipt,
    Sale,
    SaleReturn,
    SaleVoid,
    Adjustment,
    TransferOut,
    TransferIn,
}

impl MovementType {
    pub fn as_db_str(self) -> &'static str {
        return match self {
            MovementType::PurchaseReceipt => "purchase_receipt",
            MovementType::Sale => "sale",
            MovementType::SaleReturn => "sale_return",
            MovementType::SaleVoid => "sale_void",
            MovementType::Adjustment => "adjustment",
            MovementType::TransferOut => "transfer_out",
            MovementType::TransferIn => "transfer_in",
        };
    }

    /**
     * The signed ledger quantity for a line of this type.
     * Adjustments are the one type whose direction the caller decides (a count correction can go either way), so their quantity passes through as given.
     */
    pub fn signed(self, quantity: Decimal) -> Decimal {
        return match self {
            MovementType::Sale | MovementType::TransferOut => -quantity,
            MovementType::Adjustment => quantity,
            MovementType::PurchaseReceipt
            | MovementType::SaleReturn
            | MovementType::SaleVoid
            | MovementType::TransferIn => quantity,
        };
    }
}

/// What happens when an outgoing movement would take on-hand below zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegativeStockPolicy {
    /// Fail the whole posting with `PRODUCT_OUT_OF_STOCK`.
    Reject,
    /// Write it anyway; the balance goes negative.
    Allow,
    /// Write it and log a `stock_exceptions` row - the offline-sync case, where the goods have already left.
    AllowAndRecordException,
}

#[derive(Debug, Clone)]
pub struct PostingLine {
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    /// `None` means the quantity is already in the product's base unit.
    pub unit_id: Option<Uuid>,
    /// A positive magnitude, except for `Adjustment`, which is signed.
    pub quantity: Decimal,
}

pub struct Posting<'a> {
    pub tenant_id: Uuid,
    pub store_id: Uuid,
    pub movement_type: MovementType,
    pub reason: &'a str,
    pub reference_id: Option<Uuid>,
    pub actor_user_id: Uuid,
    pub policy: NegativeStockPolicy,
    /// Recorded on any `stock_exceptions` row, e.g. `offline_sale`.
    pub source: &'a str,
    pub lines: &'a [PostingLine],
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct StockMovement {
    pub id: Uuid,
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub movement_type: String,
    pub quantity: Decimal,
    pub reason: String,
}
