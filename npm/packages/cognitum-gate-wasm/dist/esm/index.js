/**
 * @cognitum/gate — ESM implementation
 *
 * Provides CognitumGate coherence verification for AI agent safety.
 * Re-exports the CJS implementation as ESM for environments that use
 * dynamic import() (e.g. nwj-write-swarm-sentinel.mjs).
 *
 * This file was added via patch-package to supply the missing dist artifacts
 * that were not included in the @cognitum/gate@0.1.0 npm publish.
 * See: NWJ patches/@cognitum+gate+0.1.0.patch
 */

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { CognitumGate } = require('../cjs/index.js');

export { CognitumGate };
export default CognitumGate;
