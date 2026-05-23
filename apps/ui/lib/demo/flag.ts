// Demo-mode flag — true when the deployment is the public, mocked-data demo.
// Read at build/runtime via NEXT_PUBLIC_DEMO_MODE.
//
// Why a dedicated module: we want a single source of truth that's safe to import
// from anywhere (server components, client components, lib code) without pulling
// React or browser-only state.

export const DEMO_MODE: boolean = (() => {
  const v = process.env.NEXT_PUBLIC_DEMO_MODE
  return v === "1" || v === "true" || v === "yes"
})()
