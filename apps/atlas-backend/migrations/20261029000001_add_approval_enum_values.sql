/*
 * FR-09 and FR-29 approval requests. Own migration: a new enum value can't be used in the
 * transaction that adds it.
 */
ALTER TYPE atlas.permission ADD VALUE 'price.approve';
ALTER TYPE atlas.notification_kind ADD VALUE 'approval_requested';
ALTER TYPE atlas.notification_kind ADD VALUE 'approval_decided';
