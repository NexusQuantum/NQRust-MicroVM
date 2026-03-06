// MDX override system for API documentation
// Override files live in content/api/overrides/{tag}/{slug}.mdx
// This module provides utilities to check for and load overrides

export interface OverrideMetadata {
  description?: string
  notes?: string
  examples?: string
}

export function getOverridePath(tagSlug: string, endpointSlug: string): string {
  return `content/api/overrides/${tagSlug}/${endpointSlug}.mdx`
}

// For future use: scan overrides directory and return a manifest
// This would be called at build time by the parse-openapi script
export function buildOverrideManifest(): Record<string, string[]> {
  // Placeholder — will be populated when MDX overrides are added
  return {}
}
