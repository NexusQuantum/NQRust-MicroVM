"use client"

import { useState, useEffect, useRef } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Search, Download, Star, TrendingUp, CheckCircle, Loader2 } from "lucide-react"
import { useSearchDockerHub, useDockerImageTags, useDownloadDockerImage } from "@/lib/queries"
import type { DockerHubImage, DownloadProgress } from "@/lib/types"
import { facadeApi } from "@/lib/api"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Progress } from "@/components/ui/progress"

export function DockerHubBrowser() {
  const [searchQuery, setSearchQuery] = useState("")
  const [searchEnabled, setSearchEnabled] = useState(false)
  const [selectedImage, setSelectedImage] = useState<DockerHubImage | null>(null)
  const [selectedTag, setSelectedTag] = useState<string>("latest")
  const [downloadDialogOpen, setDownloadDialogOpen] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null)
  const pollingIntervalRef = useRef<NodeJS.Timeout | null>(null)

  const { data: searchResults = [], isLoading: isSearching } = useSearchDockerHub(
    searchQuery,
    searchEnabled
  )
  const { data: imageTags = [], isLoading: isLoadingTags } = useDockerImageTags(
    selectedImage?.name || "",
    !!selectedImage
  )
  const downloadImage = useDownloadDockerImage()

  const handleSearch = () => {
    if (searchQuery.trim().length > 0) {
      setSearchEnabled(true)
    }
  }

  const handleDownload = (image: DockerHubImage) => {
    setSelectedImage(image)
    setSelectedTag("latest")
    setDownloadDialogOpen(true)
  }

  // Poll for download progress
  const pollProgress = async (imageName: string) => {
    try {
      const progress = await facadeApi.getDockerDownloadProgress(imageName)
      setDownloadProgress(progress)

      // Stop polling if completed or error
      if (progress.completed || progress.error) {
        console.log("Download completed/errored, stopping poll", progress)
        if (pollingIntervalRef.current) {
          clearInterval(pollingIntervalRef.current)
          pollingIntervalRef.current = null
        }

        // Auto-close dialog on completion (not on error)
        if (progress.completed && !progress.error) {
          console.log("Auto-closing dialog in 2 seconds...")
          setTimeout(() => {
            setDownloadDialogOpen(false)
            setSelectedImage(null)
            setSelectedTag("latest")
            setDownloadProgress(null)
          }, 2000)
        }
      }
    } catch (error) {
      // Progress endpoint returns 404 if not found, which is normal before download starts
      console.debug("Progress polling:", error)
    }
  }

  // Cleanup polling when dialog closes
  useEffect(() => {
    if (!downloadDialogOpen && pollingIntervalRef.current) {
      clearInterval(pollingIntervalRef.current)
      pollingIntervalRef.current = null
      setDownloadProgress(null)
    }
  }, [downloadDialogOpen])

  // Cleanup interval on unmount
  useEffect(() => {
    return () => {
      if (pollingIntervalRef.current) {
        clearInterval(pollingIntervalRef.current)
      }
    }
  }, [])

  const confirmDownload = () => {
    if (!selectedImage) return

    const imageWithTag = `${selectedImage.name}:${selectedTag}`

    // Initialize progress state
    setDownloadProgress({
      image: imageWithTag,
      status: "Starting download...",
      current_bytes: 0,
      total_bytes: 0,
      completed: false,
      error: undefined,
    })

    // Start polling for progress BEFORE starting download
    // Poll every 500ms for smoother updates
    if (pollingIntervalRef.current) {
      clearInterval(pollingIntervalRef.current)
    }
    pollingIntervalRef.current = setInterval(() => {
      pollProgress(imageWithTag)
    }, 500) // Poll every 500ms

    // Start the download (this is fire-and-forget, backend handles it)
    downloadImage.mutate(
      { image: imageWithTag },
      {
        onSuccess: () => {
          // Don't stop polling - the backend download continues
          // Keep polling until completed or error
          console.log("Download request sent, continuing to poll for progress...")
        },
        onError: (error) => {
          console.error("Download request failed:", error)
          // Stop polling on error
          if (pollingIntervalRef.current) {
            clearInterval(pollingIntervalRef.current)
            pollingIntervalRef.current = null
          }
          // Update progress with error
          setDownloadProgress((prev) => prev ? {
            ...prev,
            error: error instanceof Error ? error.message : String(error),
            completed: true,
          } : null)
        },
      }
    )
  }

  const formatNumber = (num: number) => {
    if (num >= 1000000000) return `${(num / 1000000000).toFixed(1)}B`
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`
    return num.toString()
  }

  const formatBytes = (bytes: number | undefined) => {
    if (!bytes) return "N/A"
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Docker Hub Marketplace</CardTitle>
          <CardDescription>
            Search and download container images from Docker Hub to cache locally
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Input
              placeholder="Search Docker Hub (e.g., nginx, postgres, redis)..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSearch()
              }}
            />
            <Button onClick={handleSearch} disabled={!searchQuery.trim() || isSearching}>
              {isSearching ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Searching...
                </>
              ) : (
                <>
                  <Search className="mr-2 h-4 w-4" />
                  Search
                </>
              )}
            </Button>
          </div>

          {searchEnabled && (
            <div className="space-y-4">
              {isSearching ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                </div>
              ) : searchResults.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  No images found. Try a different search term.
                </div>
              ) : (
                <div className="space-y-3">
                  {searchResults.map((image) => (
                    <Card key={image.name} className="hover:bg-accent/50 transition-colors">
                      <CardContent className="p-4">
                        <div className="flex items-start justify-between gap-4">
                          <div className="flex-1 space-y-2">
                            <div className="flex items-center gap-2">
                              <h3 className="font-semibold text-lg">{image.name}</h3>
                              {image.is_official && (
                                <Badge variant="default" className="bg-blue-500">
                                  <CheckCircle className="mr-1 h-3 w-3" />
                                  Official
                                </Badge>
                              )}
                              {image.is_automated && (
                                <Badge variant="secondary">Automated</Badge>
                              )}
                            </div>
                            <p className="text-sm text-muted-foreground line-clamp-2">
                              {image.description || "No description available"}
                            </p>
                            <div className="flex items-center gap-4 text-xs text-muted-foreground">
                              <div className="flex items-center gap-1">
                                <Star className="h-3 w-3" />
                                {formatNumber(image.star_count)} stars
                              </div>
                              <div className="flex items-center gap-1">
                                <TrendingUp className="h-3 w-3" />
                                {formatNumber(image.pull_count)} pulls
                              </div>
                            </div>
                          </div>
                          <Button
                            onClick={() => handleDownload(image)}
                            disabled={downloadImage.isPending}
                          >
                            <Download className="mr-2 h-4 w-4" />
                            Download
                          </Button>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Download Dialog */}
      <Dialog open={downloadDialogOpen} onOpenChange={setDownloadDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Download Docker Image</DialogTitle>
            <DialogDescription>
              Select a tag to download and cache this image
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Image</Label>
              <div className="flex items-center gap-2">
                <code className="bg-muted px-3 py-2 rounded text-sm flex-1">
                  {selectedImage?.name}
                </code>
                {selectedImage?.is_official && (
                  <Badge variant="default" className="bg-blue-500">
                    Official
                  </Badge>
                )}
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="tag-select">Tag</Label>
              {isLoadingTags ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading available tags...
                </div>
              ) : (
                <Select value={selectedTag} onValueChange={setSelectedTag}>
                  <SelectTrigger id="tag-select">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="max-h-[300px]">
                    {imageTags.length === 0 ? (
                      <SelectItem value="latest">latest</SelectItem>
                    ) : (
                      imageTags.slice(0, 50).map((tag) => (
                        <SelectItem key={tag.name} value={tag.name}>
                          <div className="flex items-center justify-between gap-4 w-full">
                            <span>{tag.name}</span>
                            {tag.size && (
                              <span className="text-xs text-muted-foreground">
                                {formatBytes(tag.size)}
                              </span>
                            )}
                          </div>
                        </SelectItem>
                      ))
                    )}
                  </SelectContent>
                </Select>
              )}
            </div>

            <div className="rounded-lg bg-muted p-3 text-sm">
              <p className="text-muted-foreground">
                Full image: <code className="font-mono">{selectedImage?.name}:{selectedTag}</code>
              </p>
            </div>

            {downloadProgress && !downloadProgress.completed && (
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground flex items-center gap-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {downloadProgress.status || "Downloading..."}
                  </span>
                  {downloadProgress.total_bytes > 0 ? (
                    <span className="font-medium">
                      {formatBytes(downloadProgress.current_bytes)} / {formatBytes(downloadProgress.total_bytes)}
                    </span>
                  ) : downloadProgress.current_bytes > 0 ? (
                    <span className="font-medium text-muted-foreground">
                      {formatBytes(downloadProgress.current_bytes)} downloaded
                    </span>
                  ) : null}
                </div>
                <Progress
                  value={
                    downloadProgress.total_bytes > 0
                      ? (downloadProgress.current_bytes / downloadProgress.total_bytes) * 100
                      : undefined
                  }
                  className="h-2"
                />
                {downloadProgress.total_bytes > 0 && (
                  <p className="text-xs text-muted-foreground text-center">
                    {((downloadProgress.current_bytes / downloadProgress.total_bytes) * 100).toFixed(1)}% complete
                  </p>
                )}
                {!downloadProgress.total_bytes && downloadProgress.current_bytes === 0 && (
                  <p className="text-xs text-muted-foreground text-center">
                  Initializing download...
                </p>
                )}
                {downloadProgress.error && (
                  <div className="rounded-lg bg-destructive/10 p-2">
                    <p className="text-xs text-destructive font-medium">Error: {downloadProgress.error}</p>
                  </div>
                )}
              </div>
            )}
            {downloadProgress?.completed && !downloadProgress.error && (
              <div className="rounded-lg bg-green-50 dark:bg-green-950 p-3">
                <div className="flex items-center gap-2 text-sm text-green-700 dark:text-green-400">
                  <CheckCircle className="h-4 w-4" />
                  <span className="font-medium">Download completed successfully!</span>
                </div>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                // Stop polling if active
                if (pollingIntervalRef.current) {
                  clearInterval(pollingIntervalRef.current)
                  pollingIntervalRef.current = null
                }
                setDownloadDialogOpen(false)
                setDownloadProgress(null)
              }}
              disabled={downloadImage.isPending && !downloadProgress}
            >
              {downloadProgress && !downloadProgress.completed ? "Close" : "Cancel"}
            </Button>
            <Button 
              onClick={confirmDownload} 
              disabled={(downloadImage.isPending || isLoadingTags || (downloadProgress && !downloadProgress.completed && !downloadProgress.error))}
            >
              {downloadImage.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Starting...
                </>
              ) : downloadProgress && !downloadProgress.completed && !downloadProgress.error ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Downloading...
                </>
              ) : isLoadingTags ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Loading Tags...
                </>
              ) : (
                <>
                  <Download className="mr-2 h-4 w-4" />
                  Download & Cache
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
