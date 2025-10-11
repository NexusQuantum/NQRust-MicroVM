/** @type {import('next').NextConfig} */
const nextConfig = {
  output: process.env.STATIC_EXPORT === 'true' ? 'export' : undefined,
  typedRoutes: true,
  reactStrictMode: true,
  images: {
    domains: ['localhost'],
    unoptimized: true,
  },
}

export default nextConfig
