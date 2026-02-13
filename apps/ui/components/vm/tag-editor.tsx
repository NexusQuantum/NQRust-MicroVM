"use client"

import { useState } from "react"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { X, Plus } from "lucide-react"
import { useUpdateVM } from "@/lib/queries"
import { toast } from "sonner"

interface TagEditorProps {
  vmId: string
  tags: string[]
}

export function TagEditor({ vmId, tags }: TagEditorProps) {
  const [newTag, setNewTag] = useState("")
  const updateVM = useUpdateVM()

  const handleAdd = () => {
    const trimmed = newTag.trim().toLowerCase()
    if (!trimmed) return
    if (tags.includes(trimmed)) {
      toast.error("Tag already exists")
      return
    }
    updateVM.mutate(
      { id: vmId, data: { tags: [...tags, trimmed] } },
      {
        onSuccess: () => {
          setNewTag("")
        },
        onError: (error) => {
          toast.error("Failed to add tag", {
            description: error instanceof Error ? error.message : "An unexpected error occurred",
          })
        },
      }
    )
  }

  const handleRemove = (tag: string) => {
    updateVM.mutate(
      { id: vmId, data: { tags: tags.filter((t) => t !== tag) } },
      {
        onError: (error) => {
          toast.error("Failed to remove tag", {
            description: error instanceof Error ? error.message : "An unexpected error occurred",
          })
        },
      }
    )
  }

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-1">
        {tags.map((tag) => (
          <Badge key={tag} variant="secondary" className="text-xs gap-1 pr-1">
            {tag}
            <button
              onClick={() => handleRemove(tag)}
              className="ml-0.5 rounded-full hover:bg-muted-foreground/20 p-0.5"
              disabled={updateVM.isPending}
            >
              <X className="h-2.5 w-2.5" />
            </button>
          </Badge>
        ))}
      </div>
      <div className="flex items-center gap-1">
        <Input
          value={newTag}
          onChange={(e) => setNewTag(e.target.value)}
          placeholder="Add tag..."
          className="h-7 text-xs w-40"
          onKeyDown={(e) => {
            if (e.key === "Enter") handleAdd()
          }}
        />
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={handleAdd}
          disabled={updateVM.isPending || !newTag.trim()}
        >
          <Plus className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  )
}
