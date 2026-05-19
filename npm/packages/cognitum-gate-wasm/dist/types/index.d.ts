/**
 * @cognitum/gate — TypeScript definitions
 * Patched dist artifact for @cognitum/gate@0.1.0
 */

export type Verdict = 'permit' | 'defer' | 'deny';

export interface PermitRequest {
  agentId: string;
  action: string;
  target?: string;
  context?: Record<string, unknown>;
}

export interface PermitResult {
  verdict: Verdict;
  token: string;
  coherenceScore: number;
  tileId: number;
  latencyUs: number;
  reason?: string;
  deferMs?: number;
}

export interface WitnessReceipt {
  token: string;
  witnessHash: string;
  agentId: string;
  action: string;
  target: string;
  verdict: Verdict;
  coherenceScore: number;
  timestamp: string;
  source?: string;
  outcome?: Record<string, unknown>;
  recordedAt?: string;
}

export interface GateOptions {
  tileCount?: number;
  coherenceThreshold?: number;
  policies?: unknown[];
  maxContextTokens?: number;
}

export interface GateStatus {
  tileCount: number;
  coherenceThreshold: number;
  policyCount: number;
  receiptCount: number;
  initTime: string;
  implementation: string;
  version: string;
}

export declare class CognitumGate {
  constructor(opts?: GateOptions);
  static init(opts?: GateOptions): Promise<CognitumGate>;
  permitAction(req: PermitRequest): PermitResult;
  getReceipt(token?: string): WitnessReceipt | null;
  recordOutcome(token: string, outcome: Record<string, unknown>): boolean;
  getStatus(): GateStatus;
}

export default CognitumGate;
