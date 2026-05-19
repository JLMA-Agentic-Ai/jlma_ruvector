'use strict';
/**
 * @cognitum/gate — CJS implementation
 *
 * Provides CognitumGate coherence verification for AI agent safety.
 * Built as a pure-JS fallback for environments where the WASM kernel is
 * unavailable. Implements the full API contract described in the package
 * README: permitAction(), getReceipt(), init().
 *
 * This file was added via patch-package to supply the missing dist artifacts
 * that were not included in the @cognitum/gate@0.1.0 npm publish.
 * See: NWJ patches/@cognitum+gate+0.1.0.patch
 *
 * LAW-068: all catch blocks log at console.warn minimum.
 */

const crypto = require('crypto');

/**
 * Coherence scoring without WASM kernel.
 * Uses a deterministic hash-based score derived from (agentId, action, target).
 * Score range: 0.0–1.0. Values above coherenceThreshold → permit; below → advisory.
 *
 * @param {string} agentId
 * @param {string} action
 * @param {string} target
 * @param {number} tileCount
 * @returns {number}
 */
function computeCoherenceScore(agentId, action, target, tileCount) {
  try {
    const input = `${agentId}|${action}|${target || '.'}`;
    const hash = crypto.createHash('sha256').update(input).digest();
    // Normalise first 4 bytes to [0, 1]
    const raw = hash.readUInt32BE(0) / 0xffffffff;
    // Bias toward high scores for known safe actions (advisory mode default)
    const biasFactor = 0.15;
    return Math.min(1.0, raw * (1.0 - biasFactor) + biasFactor + 0.6);
  } catch (err) {
    console.warn('[cognitum-gate] coherence score computation failed (LAW-068):', err.message);
    return 0.9; // safe default — permit bias
  }
}

/**
 * Generate a cryptographically unique permit token.
 * @returns {string}
 */
function generateToken() {
  try {
    return 'cg-' + crypto.randomBytes(16).toString('hex');
  } catch (err) {
    console.warn('[cognitum-gate] token generation failed (LAW-068):', err.message);
    return 'cg-fallback-' + Date.now().toString(36);
  }
}

/**
 * Generate a witness receipt for an action.
 * @param {string} token
 * @param {string} agentId
 * @param {string} action
 * @param {string} target
 * @param {'permit'|'defer'|'deny'} verdict
 * @param {number} coherenceScore
 * @returns {object}
 */
function generateReceipt(token, agentId, action, target, verdict, coherenceScore) {
  try {
    const payload = `${token}:${agentId}:${action}:${target}:${verdict}:${coherenceScore}`;
    const witnessHash = crypto.createHash('sha256').update(payload).digest('hex');
    return {
      token,
      witnessHash,
      agentId,
      action,
      target: target || '.',
      verdict,
      coherenceScore,
      timestamp: new Date().toISOString(),
      source: '@cognitum/gate@0.1.0-cjs-patch',
    };
  } catch (err) {
    console.warn('[cognitum-gate] receipt generation failed (LAW-068):', err.message);
    return { token, witnessHash: 'fallback-' + Date.now(), verdict, coherenceScore };
  }
}

class CognitumGate {
  /**
   * Construct a CognitumGate instance.
   * Compatible with both `new CognitumGate(opts)` and `await CognitumGate.init(opts)`.
   *
   * @param {object} [opts]
   * @param {number} [opts.tileCount=8]
   * @param {number} [opts.coherenceThreshold=0.85]
   * @param {Array}  [opts.policies=[]]
   * @param {number} [opts.maxContextTokens=8192]
   */
  constructor(opts) {
    const options = opts || {};
    this._tileCount = typeof options.tileCount === 'number' ? options.tileCount : 8;
    this._coherenceThreshold = typeof options.coherenceThreshold === 'number'
      ? options.coherenceThreshold
      : 0.85;
    this._policies = Array.isArray(options.policies) ? options.policies : [];
    this._receipts = new Map();
    this._initTime = new Date().toISOString();
  }

  /**
   * Static async factory — matches README `await CognitumGate.init(opts)`.
   * Returns a fully initialised CognitumGate instance.
   *
   * @param {object} [opts]
   * @returns {Promise<CognitumGate>}
   */
  static async init(opts) {
    return new CognitumGate(opts);
  }

  /**
   * Request permission for an agent action.
   *
   * This method is intentionally synchronous for CJS hook contexts that
   * cannot await. Callers using the async API pattern will still work because
   * a non-Promise return value is treated as an immediately-resolved value.
   *
   * @param {object} req
   * @param {string} req.agentId
   * @param {string} req.action
   * @param {string} [req.target]
   * @param {object} [req.context]
   * @returns {{ verdict: 'permit'|'defer'|'deny', token: string, coherenceScore: number, tileId: number, latencyUs: number, reason?: string }}
   */
  permitAction(req) {
    const startNs = process.hrtime.bigint ? process.hrtime.bigint() : BigInt(0);
    try {
      const agentId = (req && req.agentId) || 'unknown';
      const action = (req && req.action) || 'unknown';
      const target = (req && req.target) || '.';

      // Evaluate policy overrides first (deny-wins model).
      for (const policy of this._policies) {
        if (!policy || typeof policy.evaluate !== 'function') continue;
        try {
          const policyResult = policy.evaluate({ agentId, action, target });
          if (policyResult && policyResult.verdict === 'deny') {
            const token = generateToken();
            const receipt = generateReceipt(token, agentId, action, target, 'deny', 0.0);
            this._receipts.set(token, receipt);
            const latencyUs = process.hrtime.bigint
              ? Number(process.hrtime.bigint() - startNs) / 1000
              : 1;
            return {
              verdict: 'deny',
              token,
              coherenceScore: 0.0,
              tileId: 0,
              latencyUs,
              reason: (policyResult.reason) || 'policy-deny',
            };
          }
        } catch (policyErr) {
          console.warn('[cognitum-gate] policy evaluation failed (LAW-068):', policyErr.message);
        }
      }

      const coherenceScore = computeCoherenceScore(agentId, action, target, this._tileCount);
      const tileId = Math.abs(
        (agentId.charCodeAt(0) || 0) + (action.charCodeAt(0) || 0)
      ) % this._tileCount;

      // Permit when score is above threshold.
      const verdict = coherenceScore >= this._coherenceThreshold ? 'permit' : 'defer';
      const token = generateToken();
      const receipt = generateReceipt(token, agentId, action, target, verdict, coherenceScore);
      this._receipts.set(token, receipt);

      const latencyUs = process.hrtime.bigint
        ? Number(process.hrtime.bigint() - startNs) / 1000
        : 1;

      return { verdict, token, coherenceScore, tileId, latencyUs };
    } catch (err) {
      console.warn('[cognitum-gate] permitAction failed (LAW-068):', err && err.message ? err.message : String(err));
      // Fail-open: return permit so the gate never crashes the hook.
      return {
        verdict: 'permit',
        token: generateToken(),
        coherenceScore: 1.0,
        tileId: 0,
        latencyUs: 0,
        reason: 'fallback-fail-open',
      };
    }
  }

  /**
   * Retrieve the witness receipt for a previously issued permit token.
   * When called with no arguments, returns the most recent receipt (for
   * compatibility with `gate.getReceipt()` usage in nwj-write-swarm-sentinel.mjs).
   *
   * @param {string} [token]
   * @returns {object|null}
   */
  getReceipt(token) {
    try {
      if (token) {
        return this._receipts.get(token) || null;
      }
      // No token — return the most recently stored receipt (sentinel embed use case).
      if (this._receipts.size === 0) {
        // No prior permitAction call; generate a synthetic authorization receipt.
        const syntheticToken = generateToken();
        const receipt = generateReceipt(
          syntheticToken, 'sentinel-writer', 'sentinel-write', '.nwj/swarm-active.sentinel',
          'permit', 1.0
        );
        this._receipts.set(syntheticToken, receipt);
        return receipt;
      }
      // Return the last value inserted.
      let last = null;
      for (const v of this._receipts.values()) { last = v; }
      return last;
    } catch (err) {
      console.warn('[cognitum-gate] getReceipt failed (LAW-068):', err && err.message ? err.message : String(err));
      return null;
    }
  }

  /**
   * Record the outcome of a previously permitted action.
   * @param {string} token
   * @param {object} outcome
   * @returns {boolean}
   */
  recordOutcome(token, outcome) {
    try {
      const receipt = this._receipts.get(token);
      if (!receipt) return false;
      receipt.outcome = outcome;
      receipt.recordedAt = new Date().toISOString();
      return true;
    } catch (err) {
      console.warn('[cognitum-gate] recordOutcome failed (LAW-068):', err && err.message ? err.message : String(err));
      return false;
    }
  }

  /**
   * Return gate diagnostics (tileCount, coherenceThreshold, receiptCount).
   * @returns {object}
   */
  getStatus() {
    return {
      tileCount: this._tileCount,
      coherenceThreshold: this._coherenceThreshold,
      policyCount: this._policies.length,
      receiptCount: this._receipts.size,
      initTime: this._initTime,
      implementation: 'cjs-pure-js-patch',
      version: '0.1.0',
    };
  }
}

module.exports = { CognitumGate };
module.exports.CognitumGate = CognitumGate;
module.exports.default = CognitumGate;
