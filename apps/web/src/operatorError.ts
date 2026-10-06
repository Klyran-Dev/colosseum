/**
 * A message is safe for the primary operator surface only when the product
 * code explicitly authored it for that surface. Ordinary exceptions can carry
 * provider, transport, parser, or implementation details and must fall back
 * to the workspace's reviewed copy.
 */
export class OperatorFacingError extends Error {
  readonly operatorFacing = true;
  readonly supportReference: string | null;

  constructor(message: string, supportReference: string | null = null) {
    super(message);
    this.name = "OperatorFacingError";
    this.supportReference = supportReference;
  }
}

export function operatorFacingError(message: string, supportReference: string | null = null): OperatorFacingError {
  return new OperatorFacingError(message, supportReference);
}

export function operatorErrorMessage(cause: unknown, fallback: string): string {
  if (!(cause instanceof OperatorFacingError)) return fallback;
  return cause.supportReference ? `${fallback} (support reference: ${cause.supportReference})` : cause.message;
}
