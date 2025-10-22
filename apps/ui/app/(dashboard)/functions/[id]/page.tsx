import { FunctionEditor } from "@/components/function/function-editor"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ArrowLeft, Play, Trash2, FileText } from "lucide-react"
import Link from "next/link"

// Mock data
const mockFunction = {
  id: "fn-1",
  name: "image-processor",
  runtime: "node" as const,
  state: "active" as const,
  code: `export const handler = async (event) => {
  console.log('Processing image:', event.imageUrl);
  
  // Image processing logic here
  const result = {
    success: true,
    processedUrl: event.imageUrl + '-processed',
    timestamp: new Date().toISOString()
  };
  
  return result;
};`,
  handler: "index.handler",
  timeout_seconds: 30,
  memory_mb: 512,
  env_vars: {
    API_KEY: "sk-test-123",
    BUCKET_NAME: "my-images",
  },
  created_at: new Date(Date.now() - 86400000 * 7).toISOString(),
  updated_at: new Date(Date.now() - 3600000).toISOString(),
}

const getStatusColor = (state: string) => {
  switch (state) {
    case "active":
      return "bg-green-500/10 text-green-700 border-green-200"
    case "inactive":
      return "bg-gray-500/10 text-gray-700 border-gray-200"
    case "error":
      return "bg-red-500/10 text-red-700 border-red-200"
    default:
      return "bg-blue-500/10 text-blue-700 border-blue-200"
  }
}

export default function FunctionEditorPage({ params }: { params: { id: string } }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/functions">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold text-foreground">{mockFunction.name}</h1>
              <Badge className={getStatusColor(mockFunction.state)}>{mockFunction.state}</Badge>
            </div>
            <p className="text-sm text-muted-foreground mt-1">
              {mockFunction.runtime} • {mockFunction.memory_mb}MB • {mockFunction.timeout_seconds}s timeout
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Link href={`/functions/${params.id}/logs`}>
            <Button variant="outline" size="sm">
              <FileText className="mr-2 h-4 w-4" />
              View Logs
            </Button>
          </Link>
          <Button variant="outline" size="sm">
            <Play className="mr-2 h-4 w-4" />
            Test Function
          </Button>
          <Button variant="destructive" size="sm">
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <FunctionEditor functionData={mockFunction} />
    </div>
  )
}
