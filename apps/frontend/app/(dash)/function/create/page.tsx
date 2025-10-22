"use client";

import * as React from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import {
  CheckCircle2,
  Circle,
  Info,
  Plus,
  CircleChevronRight,
  CircleChevronDown,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import { Checkbox } from "@/components/ui/checkbox";

type CreateMode = "scratch" | "blueprint" | "image";

export default function CreateFunctionPage() {
  const router = useRouter();

  const [mode, setMode] = React.useState<CreateMode>("scratch");
  const [name, setName] = React.useState("");
  const [runtime, setRuntime] = React.useState("nodejs22.x");
  const [openRole, setOpenRole] = React.useState(false);
  const [useHostIp, setUseHostIp] = React.useState(false);

  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // TODO: sambungkan ke API backend kamu
    // contoh payload:
    const payload = {
      mode,
      name,
      runtime,
      // role config dsb…
    };
    console.log("Create function payload:", payload);
    // arahkan balik ke list (atau detail function)
    router.push("/function/list");
  };

  return (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Create function</h1>
        <div className="text-sm text-muted-foreground">
          <Link href="/function/list" className="underline underline-offset-4">
            Back to Functions
          </Link>
        </div>
      </div>

      {/* ============== Mode cards ============== */}
      <section className="rounded-lg border">
        <div className="p-4">
          <div className="mb-4 flex items-center gap-2">
            <h2 className="text-lg font-medium">Choose one of the following options to create your function.</h2>
            <Info className="h-4 w-4 text-muted-foreground" />
          </div>

          <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
            <ModeCard
              title="Author from scratch"
              desc="Start with a simple Hello World example."
              selected={mode === "scratch"}
              onClick={() => setMode("scratch")}
            />
            <ModeCard
              title="Use a blueprint"
              desc="Build a Lambda application from sample code and presets."
              selected={mode === "blueprint"}
              onClick={() => setMode("blueprint")}
            />
            <ModeCard
              title="Container image"
              desc="Select a container image to deploy for your function."
              selected={mode === "image"}
              onClick={() => setMode("image")}
            />
          </div>
        </div>
      </section>


      {/* ============== Form ============== */}
      <form onSubmit={onSubmit} className="space-y-6">
        <section className="rounded-lg border shadow-xs">
          <div className="p-4">
            <h3 className="mb-4 text-lg font-semibold">Basic information</h3>

            {/* Function name */}
            <div className="mb-5 space-y-2">
              <Label htmlFor="fn" className="">Function name</Label>
              <Input
                id="fn"
                placeholder="myFunctionName"
                value={name}
                onChange={(e) => setName(e.target.value)}
                maxLength={64}
                required
                autoComplete="off"
              />
              <p className="text-xs text-muted-foreground">
                Function name must be 1 to 64 characters, must be unique to the Region, and can’t include spaces.
                Valid characters are a–z, A–Z, 0–9, hyphens (-), and underscores (_).
              </p>
            </div>

            {/* Runtime */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Label>Runtime</Label>
                <Info className="h-3.5 w-3.5 text-muted-foreground" />
              </div>
              <div className="flex max-w-md items-center gap-2">
                <Select value={runtime} onValueChange={setRuntime}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select runtime" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="nodejs22.x">Node.js</SelectItem>
                    <SelectItem value="python3.11">Python</SelectItem>
                  </SelectContent>
                </Select>
                <Button type="button" variant="outline" size="icon" title="Refresh runtimes">
                  <RefreshIcon />
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                Choose the language to use to write your function. Note that the console code editor supports only Node.js or Python.
              </p>
            </div>
          </div>
        </section>

        <section className="rounded-lg border shadow-xs">
          <div className="p-4">
            <Collapsible open={openRole} onOpenChange={setOpenRole}>
              <CollapsibleTrigger asChild className="w-full justify-start cursor-pointer">
                <Button variant="ghost" type="button" className="px-0 font-medium text-lg flex">
                  {openRole ? <CircleChevronDown className="h-6 w-6" /> : <CircleChevronRight className="h-6 w-6" />} Network configuration
                </Button>
              </CollapsibleTrigger>
              <CollapsibleContent className="mt-3 space-y-3">
                <div className="grid gap-2 md:max-w-lg">
                  <Label htmlFor="port">Port</Label>
                  <Input id="port" placeholder="8080" />
                </div>
                {/* Use Host IP */}
                <div className="flex items-center gap-3 md:max-w-lg">
                  <Checkbox
                    id="useHostIp"
                    checked={useHostIp}
                    onCheckedChange={(v) => setUseHostIp(Boolean(v))}
                    className=""
                  />
                  <div className="grid gap-1">
                    <Label
                      htmlFor="useHostIp"
                      className="cursor-pointer"
                    >
                      Use Host IP
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      Bind container networking to the host machine’s IP (host networking).
                    </p>
                  </div>
                </div>
              </CollapsibleContent>
            </Collapsible>
          </div>
        </section>

        {/* Footer actions */}
        <div className="flex items-center justify-end gap-2">
          <Button asChild variant="outline">
            <Link href="/function/list">Cancel</Link>
          </Button>
          <Button type="submit" className="gap-2">
            <Plus className="h-4 w-4" />
            Create function
          </Button>
        </div>
      </form>
    </div>
  );
}

/* ====================== Small components ====================== */

function ModeCard({
  title,
  desc,
  selected,
  onClick,
}: {
  title: string;
  desc: string;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full cursor-pointer items-start gap-3 rounded-lg border p-4 text-left transition-colors",
        selected ? "border-primary ring-2 ring-primary/20" : "hover:bg-muted/40"
      )}
    >
      <span className="mt-1 rounded-full border p-0.5">
        {selected ? <CheckCircle2 className="h-5 w-5 text-primary" /> : <Circle className="h-5 w-5 text-muted-foreground" />}
      </span>
      <span className="space-y-1">
        <div className="font-medium">{title}</div>
        <div className="text-sm text-muted-foreground">{desc}</div>
      </span>
    </button>
  );
}

function RadioChip({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onChange}
      className={cn(
        "inline-flex items-center gap-2 rounded-lg border px-3 py-2 text-sm",
        checked ? "border-primary ring-2 ring-primary/20" : "hover:bg-muted/40"
      )}
      aria-pressed={checked}
    >
      {checked ? <CheckCircle2 className="h-4 w-4 text-primary" /> : <Circle className="h-4 w-4 text-muted-foreground" />}
      {label}
    </button>
  );
}

// simple “refresh” icon spinnerless for the runtime select
function RefreshIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M21 12a9 9 0 1 1-3-6.7" />
      <path d="M21 3v6h-6" />
    </svg>
  );
}
