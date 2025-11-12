"use client"

import * as React from "react"
import { cn } from "@/lib/utils"
import { facadeApi } from "@/lib/api"

interface AvatarProps {
  userId?: string
  avatarPath?: string
  username?: string
  size?: "sm" | "md" | "lg" | "xl"
  className?: string
}

const sizeMap = {
  sm: "h-8 w-8 text-xs",
  md: "h-10 w-10 text-sm",
  lg: "h-16 w-16 text-lg",
  xl: "h-24 w-24 text-2xl",
}

export function Avatar({ userId, avatarPath, username, size = "md", className }: AvatarProps) {
  const [imageError, setImageError] = React.useState(false)
  const [blobUrl, setBlobUrl] = React.useState<string | null>(null)

  // Generate initials from username
  const initials = React.useMemo(() => {
    if (!username) return "?"
    const parts = username.split(/[\s_-]+/)
    if (parts.length >= 2) {
      return `${parts[0][0]}${parts[1][0]}`.toUpperCase()
    }
    return username.substring(0, 2).toUpperCase()
  }, [username])

  // Fetch avatar image with authentication
  React.useEffect(() => {
    if (!avatarPath || imageError) {
      setBlobUrl(null)
      return
    }

    let objectUrl: string | null = null

    const fetchAvatar = async () => {
      try {
        const { getAuthToken } = await import("@/lib/auth/store")
        const token = getAuthToken()

        const headers: HeadersInit = {}
        if (token) {
          headers["Authorization"] = `Bearer ${token}`
        }

        const endpoint = userId
          ? facadeApi.getAvatarUrl(userId)
          : facadeApi.getMyAvatarUrl()

        const response = await fetch(endpoint, { headers })

        if (!response.ok) {
          if (response.status === 404) {
            // Avatar not found, show initials
            setBlobUrl(null)
            return
          }
          throw new Error(`Failed to fetch avatar: ${response.status}`)
        }

        const blob = await response.blob()
        objectUrl = URL.createObjectURL(blob)
        setBlobUrl(objectUrl)
      } catch (error) {
        console.error("Error fetching avatar:", error)
        setImageError(true)
        setBlobUrl(null)
      }
    }

    fetchAvatar()

    // Cleanup: revoke object URL when component unmounts or avatarPath changes
    return () => {
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl)
      }
    }
  }, [avatarPath, userId, imageError])

  // Generate a consistent color based on username
  const backgroundColor = React.useMemo(() => {
    if (!username) return "hsl(var(--muted))"

    // Simple hash function to generate consistent color
    let hash = 0
    for (let i = 0; i < username.length; i++) {
      hash = username.charCodeAt(i) + ((hash << 5) - hash)
    }

    // Generate hue between 0-360
    const hue = Math.abs(hash) % 360
    return `hsl(${hue}, 65%, 45%)`
  }, [username])

  return (
    <div
      className={cn(
        "relative inline-flex items-center justify-center rounded-full overflow-hidden flex-shrink-0",
        sizeMap[size],
        className
      )}
      style={{ backgroundColor: blobUrl ? "transparent" : backgroundColor }}
    >
      {blobUrl ? (
        <img
          src={blobUrl}
          alt={username || "User avatar"}
          className="h-full w-full object-cover"
          onError={() => setImageError(true)}
        />
      ) : (
        <span className="font-semibold text-white select-none">
          {initials}
        </span>
      )}
    </div>
  )
}

interface AvatarUploadProps {
  onUpload: (file: File) => void
  currentAvatarPath?: string
  username?: string
  isUploading?: boolean
  className?: string
}

export function AvatarUpload({
  onUpload,
  currentAvatarPath,
  username,
  isUploading = false,
  className,
}: AvatarUploadProps) {
  const fileInputRef = React.useRef<HTMLInputElement>(null)

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    // Validate file type
    if (!file.type.startsWith("image/")) {
      alert("Please select an image file")
      return
    }

    // Validate file size (max 2MB)
    if (file.size > 2 * 1024 * 1024) {
      alert("File size must be less than 2MB")
      return
    }

    onUpload(file)
  }

  const handleClick = () => {
    fileInputRef.current?.click()
  }

  return (
    <div className={cn("flex flex-col items-center gap-4", className)}>
      <div className="relative group">
        <Avatar
          avatarPath={currentAvatarPath}
          username={username}
          size="xl"
          className="transition-opacity group-hover:opacity-75"
        />
        <button
          type="button"
          onClick={handleClick}
          disabled={isUploading}
          className={cn(
            "absolute inset-0 flex items-center justify-center rounded-full",
            "bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity",
            "text-white text-sm font-medium cursor-pointer",
            "disabled:cursor-not-allowed disabled:opacity-50"
          )}
        >
          {isUploading ? "Uploading..." : "Change"}
        </button>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept="image/png,image/jpeg,image/jpg,image/webp"
        onChange={handleFileChange}
        className="hidden"
        disabled={isUploading}
      />

      <p className="text-xs text-muted-foreground text-center max-w-[200px]">
        Click to upload a new avatar (PNG, JPG, max 2MB). Image will be resized to 500x500px.
      </p>
    </div>
  )
}
