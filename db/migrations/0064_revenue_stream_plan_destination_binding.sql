-- Expand revenue-stream plans with the immutable server-owned payout route.
-- Existing rows remain readable but are intentionally ineligible for claim
-- compilation until an explicit, reviewed re-proposal binds a route.

BEGIN;

ALTER TABLE revenue_stream_plans
    ADD COLUMN IF NOT EXISTS destination_route_id uuid;

ALTER TABLE revenue_stream_plans
    ADD CONSTRAINT revenue_stream_plans_destination_route_id_uuidv7_check
    CHECK (
        destination_route_id IS NULL
        OR uuid_extract_version(destination_route_id) = 7
    );

ALTER TABLE revenue_stream_plans
    ADD CONSTRAINT revenue_stream_plans_destination_route_fk
    FOREIGN KEY (destination_route_id, merchant_id)
    REFERENCES revenue_stream_destination_routes(id, merchant_id);

CREATE INDEX IF NOT EXISTS revenue_stream_plans_destination_route_idx
    ON revenue_stream_plans (merchant_id, destination_route_id)
    WHERE destination_route_id IS NOT NULL;

COMMIT;
