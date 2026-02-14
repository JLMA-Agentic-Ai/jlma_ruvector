/**
 * Ambient module declarations for optional native/WASM backends.
 *
 * These let the SDK compile without the actual native packages installed.
 * At runtime the dynamic `import()` calls in backend.ts will resolve to the
 * real implementations (or throw, which is handled gracefully).
 */

declare module '@ruvector/rvf-node' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mod: any;
  export = mod;
}

declare module '@ruvector/rvf-wasm' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mod: any;
  export default mod;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const create: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const open: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const open_readonly: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const ingest_batch: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const query: any;
}
