import type React from "react"
import type { Metadata } from "next"

import "./globals.css"

import { Toaster } from "@/components/ui/toaster"
import { SonnerToaster } from "@/components/sonner-toaster"
import { Providers } from "./providers"

import { Montserrat, Geist as V0_Font_Geist, Geist_Mono as V0_Font_Geist_Mono, Source_Serif_4 as V0_Font_Source_Serif_4 } from 'next/font/google'

// Initialize fonts
const _geist = V0_Font_Geist({ subsets: ['latin'], weight: ["100", "200", "300", "400", "500", "600", "700", "800", "900"] })
const _geistMono = V0_Font_Geist_Mono({ subsets: ['latin'], weight: ["100", "200", "300", "400", "500", "600", "700", "800", "900"] })
const _sourceSerif_4 = V0_Font_Source_Serif_4({ subsets: ['latin'], weight: ["200", "300", "400", "500", "600", "700", "800", "900"] })

const montserrat = Montserrat({
  subsets: ["latin"],
  weight: ["300", "400", "500", "600", "700", "800"],
  variable: "--font-montserrat",
})

export const metadata: Metadata = {
  title: {
    default: "NQR-MicroVM - Virtual Machine Management Platform",
    template: "%s | NQR-MicroVM"
  },
  description: "A powerful platform for managing virtual machines, containers, and serverless functions with ease",
  keywords: ["virtual machines", "containers", "serverless", "microVM", "cloud computing", "devops"],
  authors: [{ name: "NQR-MicroVM Team" }],
  icons: {
    icon: "/favicon.ico",
    shortcut: "/favicon.ico",
    apple: "/apple-touch-icon.png",
  },
  manifest: "/site.webmanifest",
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://nqr-microvm.com",
    title: "NQR-MicroVM - Virtual Machine Management Platform",
    description: "A powerful platform for managing virtual machines, containers, and serverless functions with ease",
    siteName: "NQR-MicroVM",
  },
  twitter: {
    card: "summary_large_image",
    title: "NQR-MicroVM - Virtual Machine Management Platform",
    description: "A powerful platform for managing virtual machines, containers, and serverless functions with ease",
  },
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html: `
              (function() {
                try {
                  const theme = localStorage.getItem('nqr-microvm-theme') || 'system';
                  const isDark = theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
                  if (isDark) {
                    document.documentElement.classList.add('dark');
                  } else {
                    document.documentElement.classList.remove('dark');
                  }
                } catch (e) {}
              })();
            `,
          }}
        />
      </head>
      <body className={`${montserrat.variable} font-sans antialiased bg-background text-foreground`}>
        <Providers>
          {children}
          <Toaster />
          <SonnerToaster />
        </Providers>
      </body>
    </html>
  )
}
