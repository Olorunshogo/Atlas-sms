/*
 * Platform staff have no tenant. The invite row still has to exist so a
 * new email can accept a link, so tenant_id is optional and store_id must
 * stay empty on that path.
 */
ALTER TABLE atlas.invites ALTER COLUMN tenant_id DROP NOT NULL;

ALTER TABLE atlas.invites
    ADD CONSTRAINT invites_platform_or_tenant CHECK (
        (tenant_id IS NULL AND store_id IS NULL) OR tenant_id IS NOT NULL
    );
