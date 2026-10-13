-- ADR-0505: bind invited sign-up challenges to the initiating invitation.
BEGIN;

ALTER TABLE auth_otp_challenges
    ADD COLUMN IF NOT EXISTS invitation_id uuid REFERENCES auth_tenant_invitations(id);

CREATE INDEX IF NOT EXISTS auth_otp_challenges_invitation_idx
    ON auth_otp_challenges (invitation_id, created_at DESC)
    WHERE invitation_id IS NOT NULL;

COMMIT;
