"use client"

/**
 * Renders EULA plain-text with bolded article/section titles.
 * Title detection: lines that are ALL-CAPS words (ignoring punctuation/numbers/dashes/spaces).
 */
function isTitle(line: string): boolean {
    const trimmed = line.trim()
    if (!trimmed) return false
    // Strip non-alpha characters, check that all alpha chars are uppercase
    const alphaOnly = trimmed.replace(/[^a-zA-Z]/g, "")
    return alphaOnly.length > 0 && alphaOnly === alphaOnly.toUpperCase()
}

export function EulaContent({ content }: { content: string }) {
    const lines = content.split("\n")

    return (
        <div className="px-6 py-4 text-sm leading-relaxed text-foreground space-y-0.5">
            {lines.map((line, i) => {
                const trimmed = line.trim()
                if (!trimmed) {
                    return <div key={i} className="h-2" />
                }
                if (isTitle(trimmed)) {
                    return (
                        <p key={i} className="font-bold text-foreground mt-4 mb-1">
                            {line}
                        </p>
                    )
                }
                return (
                    <p key={i} className="text-muted-foreground whitespace-pre-wrap">
                        {line}
                    </p>
                )
            })}
        </div>
    )
}
