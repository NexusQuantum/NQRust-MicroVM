"use client"

import type React from "react"

import { useState, useEffect } from "react"
import { useRouter } from "next/navigation"
import Image from "next/image"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { authApi } from "@/lib/api/auth"
import { useAuthStore } from "@/lib/auth/store"
import { parseFacadeError } from "@/lib/api"
import { Eye, EyeOff } from "lucide-react"
import { toast } from "sonner"

export default function LandingPage() {
  const router = useRouter()
  const { isAuthenticated, setAuth } = useAuthStore()
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [isLoading, setIsLoading] = useState(false)
  const [isCheckingAuth, setIsCheckingAuth] = useState(true)
  const [showPassword, setShowPassword] = useState(false)

  // Check authentication status on mount
  useEffect(() => {
    // Small delay to ensure auth store is hydrated from localStorage
    const timer = setTimeout(() => {
      setIsCheckingAuth(false)
      if (isAuthenticated) {
        router.replace("/dashboard")
      }
    }, 100)

    return () => clearTimeout(timer)
  }, [isAuthenticated, router])

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault()

    if (!username || !password) {
      toast.error("Validation Error", {
        description: "Please enter both username and password",
      })
      return
    }

    setIsLoading(true)

    try {
      const response = await authApi.login({ username, password })
      setAuth(response.token, response.user)
      router.push("/dashboard")
      toast.success("Login Successful", {
        description: `Welcome back, ${response.user.username}!`,
      })
    } catch (error) {
      const facadeError = parseFacadeError(error)

      // Handle specific error cases
      if (facadeError) {
        // Check for authentication errors (401)
        if (facadeError.status === 401) {
          toast.error("Invalid Credentials", {
            description: "The username or password you entered is incorrect. Please try again.",
          })
        }
        // Check for other specific errors
        else {
          toast.error(facadeError.error || "Login Failed", {
            description: facadeError.suggestion || facadeError.fault_message || "Please check your credentials and try again",
          })
        }
      } else {
        // Generic error for network issues or unexpected errors
        toast.error("Login Failed", {
          description: "Unable to connect to the server. Please check your connection and try again.",
        })
      }
    } finally {
      setIsLoading(false)
    }
  }

  // Show loading spinner while checking authentication
  if (isCheckingAuth) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600"></div>
      </div>
    )
  }

  return (
    <div className="min-h-screen flex">
      {/* Left Side - Branding with Video Background */}
      <div className="hidden lg:flex lg:w-1/2 relative p-12 flex-col justify-between overflow-hidden">
        {/* Video Background */}
        <video
          autoPlay
          loop
          muted
          playsInline
          className="absolute inset-0 w-full h-full object-cover"
        >
          <source src="/background.mp4" type="video/mp4" />
          {/* Fallback for browsers that don't support video */}
          Your browser does not support the video tag.
        </video>

        {/* Overlay for better text readability (optional) */}
        <div className="absolute inset-0 bg-black/40"></div>

        {/* Content on top of video */}
        <div className="relative z-10">
          <div className="mb-8">
            <Image src="/nqr-logo-full.png" alt="NQR-MicroVM" width={73} height={62} priority />
          </div>
        </div>

        <div className="text-white/90 relative z-10">
          <p className="text-lg italic leading-relaxed">
            Nexus Quantum Technologies delivers the world's first vertically integrated, Rust-powered cloud platform designed for the Agentic AI era.
          </p>
          <p className="mt-4 font-medium">-Nexus Quantum Rust</p>
        </div>
      </div>

      {/* Right Side - Login Form */}
      <div className="w-full lg:w-1/2 flex items-center justify-center p-8 bg-muted dark:bg-black">
        <div className="w-full max-w-lg">
          {/* Mobile Logo */}
          <div className="lg:hidden mb-8 flex justify-center">
            <Image src="/nqr-logo-full.png" alt="NQR-MicroVM" width={280} height={80} priority />
          </div>

          <Card className="border-0 shadow-lg p-8">
            <CardHeader className="space-y-1 px-0">
              <CardTitle className="text-2xl font-bold mx-auto">Sign In</CardTitle>
              <CardTitle className="text-md font-medium mx-auto text-muted-foreground">Welcome back! Please enter your credentials to continue.</CardTitle>
            </CardHeader>
            <CardContent className="px-0">
              <form onSubmit={handleLogin} className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="username">Username</Label>
                  <Input
                    autoComplete="off"
                    id="username"
                    type="text"
                    placeholder="Enter your username"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    disabled={isLoading}
                    className="h-11"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="password">Password</Label>
                  <div className="relative">
                    <Input
                      autoComplete="off"
                      id="password"
                      type={showPassword ? "text" : "password"}
                      placeholder="Enter your password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      disabled={isLoading}
                      className="h-11 pr-10"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                      onClick={() => setShowPassword(!showPassword)}
                      disabled={isLoading}
                    >
                      {showPassword ? (
                        <Eye className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <EyeOff className="h-4 w-4 text-muted-foreground" />
                      )}
                    </Button>
                  </div>
                </div>

                <Button
                  type="submit"
                  className="w-full h-11 bg-orange-600 hover:bg-orange-700 text-white font-medium cursor-pointer"
                  disabled={isLoading}
                >
                  {isLoading ? "Signing in..." : "Sign In"}
                </Button>
              </form>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
