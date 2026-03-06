"use client"

import { useState, useEffect, useMemo } from "react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { CopyButton } from "./copy-button"
import type { ParsedEndpoint } from "@/lib/docs/openapi-types"
import { generateCurl, generateJavaScript, generatePython } from "@/lib/docs/code-gen"

function useHighlighter() {
  const [highlighter, setHighlighter] = useState<{
    codeToHtml: (code: string, opts: { lang: string; theme: string }) => string
  } | null>(null)

  useEffect(() => {
    let cancelled = false
    import("shiki").then(({ createHighlighter }) =>
      createHighlighter({
        themes: ["github-dark", "github-light"],
        langs: ["bash", "javascript", "python", "json"],
      })
    ).then((h) => {
      if (!cancelled) setHighlighter(h)
    })
    return () => { cancelled = true }
  }, [])

  return highlighter
}

function HighlightedCode({
  code,
  lang,
  highlighter,
}: {
  code: string
  lang: string
  highlighter: ReturnType<typeof useHighlighter>
}) {
  const html = useMemo(() => {
    if (!highlighter) return null
    return highlighter.codeToHtml(code, { lang, theme: "github-dark" })
  }, [code, lang, highlighter])

  if (!html) {
    return (
      <pre className="overflow-x-auto rounded-lg bg-zinc-900 p-4 text-sm text-zinc-100">
        <code>{code}</code>
      </pre>
    )
  }

  return (
    <div
      className="overflow-x-auto rounded-lg text-sm [&_pre]:p-4"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}

const LANGS = [
  { key: "curl", label: "cURL", lang: "bash" },
  { key: "javascript", label: "JavaScript", lang: "javascript" },
  { key: "python", label: "Python", lang: "python" },
] as const

export function CodeExample({ endpoint }: { endpoint: ParsedEndpoint }) {
  const highlighter = useHighlighter()

  const examples = useMemo(
    () => ({
      curl: generateCurl(endpoint),
      javascript: generateJavaScript(endpoint),
      python: generatePython(endpoint),
    }),
    [endpoint]
  )

  return (
    <Tabs defaultValue="curl" className="w-full">
      <div className="flex items-center justify-between rounded-t-lg border border-b-0 border-border bg-zinc-900 px-3">
        <TabsList className="h-9 border-0 bg-transparent p-0">
          {LANGS.map((l) => (
            <TabsTrigger
              key={l.key}
              value={l.key}
              className="rounded-none border-b-2 border-transparent px-3 py-1.5 text-xs text-zinc-400 data-[state=active]:border-orange-500 data-[state=active]:bg-transparent data-[state=active]:text-white data-[state=active]:shadow-none"
            >
              {l.label}
            </TabsTrigger>
          ))}
        </TabsList>
      </div>
      {LANGS.map((l) => (
        <TabsContent key={l.key} value={l.key} className="mt-0">
          <div className="relative">
            <CopyButton
              value={examples[l.key]}
              className="absolute right-2 top-2 text-zinc-400 hover:text-white"
            />
            <HighlightedCode
              code={examples[l.key]}
              lang={l.lang}
              highlighter={highlighter}
            />
          </div>
        </TabsContent>
      ))}
    </Tabs>
  )
}

export { useHighlighter, HighlightedCode }
