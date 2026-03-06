"use client"

import { useEffect, useState } from "react"
import { useRouter } from "next/navigation"
import { useEulaInfo, useAcceptEula } from "@/lib/queries"
import { useAuthStore } from "@/lib/auth/store"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { Loader2, Globe } from "lucide-react"
import { toast } from "sonner"
import { EulaContent } from "@/components/license/eula-content"
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select"

export default function EulaPage() {
    const { data: eulaInfo, isLoading: isInfoLoading } = useEulaInfo()
    const acceptMutation = useAcceptEula()
    const { isAuthenticated, token } = useAuthStore()
    const router = useRouter()

    const [language, setLanguage] = useState<string>("en")
    const [content, setContent] = useState<string>("")
    const [isLoadingContent, setIsLoadingContent] = useState<boolean>(true)
    const [isAccepted, setIsAccepted] = useState<boolean>(false)

    // Fetch EULA markdown content from static folder
    useEffect(() => {
        setIsLoadingContent(true)
        let fileName = "EULA.md"
        if (language === "id") {
            fileName = "EULA_id.md"
        }

        fetch(`/eula/${fileName}`)
            .then((res) => {
                if (!res.ok) throw new Error("Could not load EULA document")
                return res.text()
            })
            .then((text) => {
                setContent(text)
                setIsLoadingContent(false)
            })
            .catch((err) => {
                console.error("Failed to load EULA:", err)
                setContent("# Error loading EULA\n\nPlease contact the administrator.")
                setIsLoadingContent(false)
            })
    }, [language])

    const handleAccept = () => {
        if (!eulaInfo?.version) return

        acceptMutation.mutate(
            { version: eulaInfo.version, language },
            {
                onSuccess: () => {
                    toast.success("End User License Agreement accepted")
                    router.replace(isAuthenticated && token ? "/dashboard" : "/")
                },
                onError: (err: any) => {
                    toast.error("Failed to accept EULA", {
                        description: err?.message || "Please try again later"
                    })
                }
            }
        )
    }

    if (isInfoLoading) {
        return (
            <Card className="w-full max-w-3xl border-muted bg-card text-card-foreground">
                <CardContent className="flex items-center justify-center py-12">
                    <Loader2 className="h-8 w-8 animate-spin text-primary" />
                </CardContent>
            </Card>
        )
    }

    return (
        <Card className="w-full max-w-4xl border-muted bg-card text-card-foreground shadow-lg flex flex-col h-[85vh]">
            <CardHeader className="flex-none border-b border-border/40 pb-4">
                <div className="flex items-center justify-between">
                    <div>
                        <CardTitle className="text-2xl">End User License Agreement</CardTitle>
                        <CardDescription className="text-muted-foreground mt-1">
                            Please read and accept the terms to continue using NQRust-MicroVM.
                            {eulaInfo?.version && <span className="ml-2 inline-flex items-center rounded-md bg-secondary px-2 py-0.5 text-xs font-medium text-secondary-foreground ring-1 ring-inset ring-secondary-foreground/10">v{eulaInfo.version}</span>}
                        </CardDescription>
                    </div>
                    <div className="flex items-center gap-2">
                        <Globe className="h-4 w-4 text-muted-foreground" />
                        <Select value={language} onValueChange={setLanguage}>
                            <SelectTrigger className="w-[140px] h-8 text-xs">
                                <SelectValue placeholder="Language" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="en">English</SelectItem>
                                <SelectItem value="id">Bahasa Indonesia</SelectItem>
                            </SelectContent>
                        </Select>
                    </div>
                </div>
            </CardHeader>

            <CardContent className="flex-grow overflow-hidden p-0">
                <div className="h-full bg-muted/20 relative">
                    {isLoadingContent ? (
                        <div className="absolute inset-0 flex items-center justify-center bg-background/50 backdrop-blur-sm z-10">
                            <Loader2 className="h-8 w-8 animate-spin text-primary" />
                        </div>
                    ) : null}
                    <ScrollArea className="h-full w-full">
                        <EulaContent content={content} />
                    </ScrollArea>
                </div>
            </CardContent>

            <CardFooter className="flex-none flex-col gap-4 border-t border-border/40 pt-4 bg-background/50">
                <div className="flex items-center space-x-2 w-full justify-start mt-2">
                    <Checkbox
                        id="terms"
                        checked={isAccepted}
                        onCheckedChange={(checked) => setIsAccepted(checked as boolean)}
                        className="border-primary/50 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground focus-visible:ring-primary"
                    />
                    <Label
                        htmlFor="terms"
                        className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer select-none"
                    >
                        I have read and agree to the End User License Agreement
                    </Label>
                </div>
                <div className="flex justify-end w-full">
                    <Button
                        disabled={!isAccepted || acceptMutation.isPending}
                        onClick={handleAccept}
                        className="min-w-[120px]"
                    >
                        {acceptMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                        I Accept
                    </Button>
                </div>
            </CardFooter>
        </Card>
    )
}
