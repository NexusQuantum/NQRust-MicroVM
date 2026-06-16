/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
  // noVNC ships as raw browser ESM under core/ — Next must transpile it.
  transpilePackages: ["@novnc/novnc"],
}

export default nextConfig
