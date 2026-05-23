/** @type {import('next').NextConfig} */
// Standalone output is for self-hosted runtimes (the manager bundles
// .next/standalone). Vercel uses its own serverless runtime and emits a
// warning when output: standalone is set, so we disable it there.
const isVercel = !!process.env.VERCEL

const nextConfig = {
  ...(isVercel ? {} : { output: "standalone" }),
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
}

export default nextConfig
