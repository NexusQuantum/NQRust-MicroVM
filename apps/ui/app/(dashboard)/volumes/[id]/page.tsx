"use client";

import { use } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft, HardDrive, Loader2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { ReusableTabs, TabItem, TabContentItem } from "@/components/dashboard/tabs-new";
import { VolumeBackupsTab } from "@/components/volume/volume-backups-tab";
import { useVolume } from "@/lib/queries";

function getStatusColor(status: string) {
  switch (status) {
    case "available":
      return "bg-green-500/10 text-green-700 border-green-200";
    case "attached":
      return "bg-blue-500/10 text-blue-700 border-blue-200";
    case "creating":
      return "bg-yellow-500/10 text-yellow-700 border-yellow-200";
    case "error":
      return "bg-red-500/10 text-red-700 border-red-200";
    default:
      return "bg-gray-500/10 text-gray-700 border-gray-200";
  }
}

export default function VolumeDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const searchParams = useSearchParams();
  const tabParam = searchParams.get("tab");

  const validTabs = ["overview", "backups"];
  const defaultTab =
    tabParam && validTabs.includes(tabParam) ? tabParam : "overview";

  const { data: volume, isLoading, error } = useVolume(id);

  const tabs: TabItem[] = [
    { value: "overview", label: "Overview" },
    { value: "backups", label: "Backups" },
  ];

  const contents: TabContentItem[] = [
    {
      value: "overview",
      content: volume ? (
        <div className="space-y-4">
          <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm max-w-lg">
            <dt className="text-muted-foreground">ID</dt>
            <dd className="font-mono text-xs">{volume.id}</dd>
            <dt className="text-muted-foreground">Name</dt>
            <dd>{volume.name}</dd>
            {volume.description && (
              <>
                <dt className="text-muted-foreground">Description</dt>
                <dd>{volume.description}</dd>
              </>
            )}
            <dt className="text-muted-foreground">Status</dt>
            <dd>
              <Badge className={getStatusColor(volume.status)} variant="outline">
                {volume.status}
              </Badge>
            </dd>
            <dt className="text-muted-foreground">Type</dt>
            <dd>{volume.type}</dd>
            <dt className="text-muted-foreground">Size</dt>
            <dd>{volume.size_gb} GB</dd>
            <dt className="text-muted-foreground">Created</dt>
            <dd>{new Date(volume.created_at).toLocaleString()}</dd>
          </dl>
        </div>
      ) : null,
    },
    {
      value: "backups",
      content: <VolumeBackupsTab volumeId={id} />,
    },
  ];

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !volume) {
    return (
      <div className="space-y-4">
        <Button variant="ghost" asChild>
          <Link href="/volumes">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Volumes
          </Link>
        </Button>
        <Alert variant="destructive">
          <AlertDescription>Failed to load volume.</AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="sm" asChild>
          <Link href="/volumes">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Volumes
          </Link>
        </Button>
        <div className="flex items-center gap-2">
          <HardDrive className="h-5 w-5 text-muted-foreground" />
          <h1 className="text-2xl font-bold">{volume.name}</h1>
          <Badge className={getStatusColor(volume.status)} variant="outline">
            {volume.status}
          </Badge>
        </div>
      </div>

      <ReusableTabs
        tabs={tabs}
        contents={contents}
        defaultValue={defaultTab}
      />
    </div>
  );
}
