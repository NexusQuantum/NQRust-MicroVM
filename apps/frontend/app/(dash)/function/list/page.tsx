"use client";

import * as React from "react";
import Link from "next/link";
import {
  RefreshCw,
  Plus,
  ChevronLeft,
  ChevronRight,
  MoreHorizontal,
  Trash,
  Download,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { cn } from "@/lib/utils";

// ===== Types =====
type FnItem = {
  id: string;
  name: string;
  description?: string;
  packageType: "Zip" | "Image";
  runtime: string; // e.g. "Node.js 22.x"
  lastModified: string; // ISO date
};

// ===== Mock data (ganti dengan fetch ke API kamu) =====
const MOCK: FnItem[] = [
  {
    id: "tes",
    name: "tes",
    description: "",
    packageType: "Zip",
    runtime: "Node.js",
    lastModified: new Date(Date.now() - 7 * 24 * 3600 * 1000).toISOString(),
  },
];

// helpers
function timeAgo(dateISO: string) {
  const s = Math.floor((Date.now() - new Date(dateISO).getTime()) / 1000);
  const rtf = new Intl.RelativeTimeFormat("en", { numeric: "auto" });
  if (s < 60) return rtf.format(-s, "seconds");
  const m = Math.floor(s / 60);
  if (m < 60) return rtf.format(-m, "minutes");
  const h = Math.floor(m / 60);
  if (h < 24) return rtf.format(-h, "hours");
  const d = Math.floor(h / 24);
  return rtf.format(-d, "days");
}

export default function FunctionListPage() {
  const [items, setItems] = React.useState<FnItem[]>(MOCK);
  const [q, setQ] = React.useState("");
  const [lastFetched, setLastFetched] = React.useState<Date>(new Date());
  const [page, setPage] = React.useState(1);
  const pageSize = 10;

  // contoh fetcher (tinggal sambungkan ke API kamu)
  const fetchData = React.useCallback(async () => {
    // const res = await fetch("/api/functions");
    // const data: FnItem[] = await res.json();
    const data = MOCK; // sementara
    setItems(data);
    setLastFetched(new Date());
  }, []);

  React.useEffect(() => {
    fetchData();
  }, [fetchData]);

  const filtered = React.useMemo(() => {
    const qq = q.trim().toLowerCase();
    if (!qq) return items;
    return items.filter(
      (x) =>
        x.name.toLowerCase().includes(qq) ||
        x.runtime.toLowerCase().includes(qq) ||
        (x.description ?? "").toLowerCase().includes(qq)
    );
  }, [items, q]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / pageSize));
  const slice = filtered.slice((page - 1) * pageSize, page * pageSize);

  React.useEffect(() => {
    // reset ke halaman 1 kalau filter berubah
    setPage(1);
  }, [q]);

  return (
    <div className="flex h-full flex-col gap-4">
      {/* Top bar */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-semibold">
            Functions{" "}
            <span className="text-muted-foreground text-base">
              ({filtered.length})
            </span>
          </h1>
          <span className="text-muted-foreground text-sm">
            Last fetched {timeAgo(lastFetched.toISOString())}
          </span>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={fetchData} className="gap-2">
            <RefreshCw className="h-4 w-4" />
            Refresh
          </Button>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm" className="gap-2 hover:cursor-pointer">
                Actions
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => alert("Delete selected (stub)")} className="hover:cursor-pointer">
                <Trash />
                Delete
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => alert("Export (stub)")} className="hover:cursor-pointer">
                <Download />
                Export
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          <Button asChild variant={'default'} size="sm" className="gap-2">
            <Link href={{ pathname: "/function/create" }}>
              <Plus className="h-4 w-4" />
              Create function
            </Link>
          </Button>
        </div>
      </div>

      {/* Filter bar */}
      <div className="flex items-center gap-3">
        <Input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Filter by attributes or search by keyword"
          className="max-w-xl"
        />
      </div>

      {/* Table */}
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-10"></TableHead>
              <TableHead>Function name</TableHead>
              <TableHead>Description</TableHead>
              <TableHead>Package type</TableHead>
              <TableHead>Runtime</TableHead>
              <TableHead>Last modified</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {slice.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} className="h-24 text-center text-sm text-muted-foreground">
                  No functions found.
                </TableCell>
              </TableRow>
            )}

            {slice.map((fn) => (
              <TableRow key={fn.id} className="hover:bg-muted/40">
                <TableCell>
                  <input type="checkbox" aria-label={`select ${fn.name}`} />
                </TableCell>
                <TableCell className="font-medium">
                  <Link
                    // href={(`/function/${encodeURIComponent(fn.id)}`) as unknown as any}
                    href='/function/lambda'
                    className="text-primary underline-offset-2 hover:underline"
                  >
                    {fn.name}
                  </Link>
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {fn.description || "-"}
                </TableCell>
                <TableCell>{fn.packageType}</TableCell>
                <TableCell>{fn.runtime}</TableCell>
                <TableCell className="whitespace-nowrap">
                  {timeAgo(fn.lastModified)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-end gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPage((p) => Math.max(1, p - 1))}
          disabled={page <= 1}
          className="gap-1"
        >
          <ChevronLeft className="h-4 w-4" />
          Prev
        </Button>
        <span className="text-sm text-muted-foreground">
          Page <span className="font-medium">{page}</span> of{" "}
          <span className="font-medium">{totalPages}</span>
        </span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
          disabled={page >= totalPages}
          className="gap-1"
        >
          Next
          <ChevronRight className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
