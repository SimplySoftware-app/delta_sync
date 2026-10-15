import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  metadataBase: new URL("https://deltasync.simplysoftware.app"),
  title: "DeltaSync - Authentic Content Protocol",
  description: "A revolutionary protocol ensuring video authenticity in the age of AI-generated content. Verify real videos from trusted devices.",
  keywords: ["video authenticity", "AI content detection", "blockchain", "content verification", "deepfake prevention"],
  alternates: {
    canonical: "/",
  },
  openGraph: {
    title: "DeltaSync - Authentic Content Protocol",
    description: "A revolutionary protocol ensuring video authenticity in the age of AI-generated content.",
    url: "https://deltasync.simplysoftware.app",
    siteName: "DeltaSync",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "DeltaSync - Authentic Content Protocol",
    description: "A revolutionary protocol ensuring video authenticity in the age of AI-generated content.",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="scroll-smooth">
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
