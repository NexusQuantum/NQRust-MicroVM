export default function EulaLayout({
    children,
}: {
    children: React.ReactNode
}) {
    return (
        <div className="min-h-screen bg-background font-sans antialiased text-foreground flex items-center justify-center p-4">
            {children}
        </div>
    )
}
