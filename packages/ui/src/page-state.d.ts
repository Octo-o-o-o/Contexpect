export declare const STATES: readonly string[];
export interface StateVerdict {
  state: string;
  reasonCode: string;
  retryable: boolean;
}
export declare function classifyFailure(kind: "transport" | "envelope", code: string): StateVerdict;
export declare function classifyPayload(data: unknown): StateVerdict;
