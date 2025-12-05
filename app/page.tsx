'use client';

import { Lock, Layers, Globe, CheckCircle, AlertTriangle, Video, Smartphone } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import Logo from './components/Logo';

export default function Home() {
  return (
    <div className="min-h-screen bg-[#eee9df]">
      {/* Navigation */}
      <nav className="fixed top-0 w-full bg-[#eee9df]/80 backdrop-blur-md z-50 border-b border-[#2c3b4d]/20">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center space-x-2">
              <Logo size={32} />
              <span className="text-2xl font-bold text-[#2c3b4d]">
                DeltaSync
              </span>
            </div>
            <div className="hidden md:flex space-x-8">
              <a href="#problem" className="text-[#1C1C1E] hover:text-[#2c3b4d] transition">Problem</a>
              <a href="#solution" className="text-[#1C1C1E] hover:text-[#2c3b4d] transition">Solution</a>
              <a href="#principles" className="text-[#1C1C1E] hover:text-[#2c3b4d] transition">Principles</a>
            </div>
            <Button>
              Get Started
            </Button>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="pt-32 pb-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="text-center animate-fade-in">
            <Badge variant="secondary" className="mb-6">
              <Video className="h-4 w-4" />
              <span className="ml-2">Revolutionizing Content Authenticity</span>
            </Badge>
            <h1 className="text-5xl md:text-7xl font-bold text-[#1C1C1E] mb-6 leading-tight">
              Authentic Content in the
              <span className="block text-[#2c3b4d]">
                Age of AI
              </span>
            </h1>
            <p className="text-xl md:text-2xl text-[#1C1C1E]/80 mb-12 max-w-3xl mx-auto">
              A revolutionary protocol that ensures video authenticity through cryptographic verification, 
              protecting viewers from AI-generated deception. Built on C2PA standards and leveraging 
              distributed ledger technology (DLT) for Authentication of Media via Provenance (AMP).
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Button size="lg">
                Join the Waitlist
              </Button>
              <Button variant="outline" size="lg">
                Learn More
              </Button>
            </div>
          </div>
        </div>
      </section>

      {/* Problem Section */}
      <section id="problem" className="py-20 px-4 sm:px-6 lg:px-8 bg-white">
        <div className="max-w-7xl mx-auto">
          <div className="text-center mb-16">
            <Badge variant="destructive" className="mb-4">
              <AlertTriangle className="h-4 w-4" />
              <span className="ml-2">The Growing Crisis</span>
            </Badge>
            <h2 className="text-4xl md:text-5xl font-bold text-[#1C1C1E] mb-6">
              The AI Content Problem
            </h2>
            <p className="text-xl text-[#1C1C1E]/80 max-w-3xl mx-auto">
              {`We're living in an era where AI-generated content is flooding the internet, 
              exploiting our dopamine systems and eroding trust in digital media.`}
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8 mb-12">
            <Card className="bg-gradient-to-br from-[#a35139]/10 to-[#a35139]/5 border-[#a35139]/20">
              <CardHeader>
                <div className="bg-[#a35139]/20 w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <Video className="h-6 w-6 text-[#a35139]" />
                </div>
                <CardTitle>Fake Engagement</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Content creators are generating fake videos to manipulate engagement and monetize attention 
                  through deceptive means.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#a35139]/10 to-[#a35139]/5 border-[#a35139]/20">
              <CardHeader>
                <div className="bg-[#a35139]/20 w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <AlertTriangle className="h-6 w-6 text-[#a35139]" />
                </div>
                <CardTitle>Trust Erosion</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  As AI becomes more sophisticated, distinguishing real from fake becomes impossible, 
                  leading to widespread distrust.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#a35139]/10 to-[#a35139]/5 border-[#a35139]/20">
              <CardHeader>
                <div className="bg-[#a35139]/20 w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <Globe className="h-6 w-6 text-[#a35139]" />
                </div>
                <CardTitle>Platform Exodus</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Without solutions, users will abandon platforms entirely, and authentic creators 
                  will lose their audiences.
                </p>
              </CardContent>
            </Card>
          </div>

          <Card className="bg-gradient-to-r from-[#1C1C1E] to-[#2c3b4d] text-white border-0">
            <CardContent className="pt-6">
              <p className="text-xl md:text-2xl font-semibold text-center">
                {`"By 2026, if we don't act now, authentic content will be indistinguishable from AI-generated media, 
                causing a mass exodus from social platforms."`}
              </p>
            </CardContent>
          </Card>
        </div>
      </section>

      {/* Solution Section */}
      <section id="solution" className="py-20 px-4 sm:px-6 lg:px-8 bg-[#eee9df]">
        <div className="max-w-7xl mx-auto">
          <div className="text-center mb-16">
            <Badge variant="default" className="mb-4">
              <CheckCircle className="h-4 w-4" />
              <span className="ml-2">Our Solution</span>
            </Badge>
            <h2 className="text-4xl md:text-5xl font-bold text-[#1C1C1E] mb-6">
              Introducing DeltaSync Protocol
            </h2>
            <p className="text-xl text-[#1C1C1E]/80 max-w-3xl mx-auto">
              A cryptographic protocol that creates an unbreakable chain of authenticity from device 
              to platform, ensuring every video can be verified as genuine. Built on C2PA (Coalition for Content Provenance and Authenticity) 
              standards and powered by distributed ledger technology (DLT) for Authentication of Media via Provenance (AMP).
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-12 items-center">
            <div>
              <h3 className="text-3xl font-bold text-[#1C1C1E] mb-6">How It Works</h3>
              <div className="space-y-6">
                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-[#2c3b4d] text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      1
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-[#1C1C1E] mb-2">Device Capture</h4>
                    <p className="text-[#1C1C1E]/80">
                      Videos are cryptographically signed at the moment of capture by trusted devices 
                      (iPhone, Pixel, Samsung, etc.) using C2PA-compliant protocols for Authentication of Media via Provenance (AMP).
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-[#2c3b4d] text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      2
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-[#1C1C1E] mb-2">Unique Fingerprint</h4>
                    <p className="text-[#1C1C1E]/80">
                      Each video receives a unique cryptographic hash that remains attached through downloads 
                      and shares.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-[#2c3b4d] text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      3
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-[#1C1C1E] mb-2">Platform Verification</h4>
                    <p className="text-[#1C1C1E]/80">
                      Social platforms can instantly verify authenticity and label content accordingly, 
                      giving users confidence.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-[#2c3b4d] text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      4
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-[#1C1C1E] mb-2">User Control</h4>
                    <p className="text-[#1C1C1E]/80">
                      Users can filter their feeds to show only verified authentic content, eliminating 
                      AI-generated spam.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <Card className="shadow-xl">
              <CardContent className="pt-6">
                <div className="aspect-square bg-gradient-to-br from-[#2c3b4d]/10 to-[#a35139]/10 rounded-xl flex items-center justify-center mb-6">
                  <Logo size={128} />
                </div>
                <div className="space-y-4">
                  <div className="flex items-center justify-between p-4 bg-[#a35139]/10 rounded-lg">
                    <span className="font-semibold text-[#1C1C1E]">Authenticity</span>
                    <CheckCircle className="h-6 w-6 text-[#a35139]" />
                  </div>
                  <div className="flex items-center justify-between p-4 bg-[#2c3b4d]/10 rounded-lg">
                    <span className="font-semibold text-[#1C1C1E]">Privacy Protected</span>
                    <Lock className="h-6 w-6 text-[#2c3b4d]" />
                  </div>
                  <div className="flex items-center justify-between p-4 bg-[#2c3b4d]/10 rounded-lg">
                    <span className="font-semibold text-[#1C1C1E]">DLT Verified</span>
                    <Layers className="h-6 w-6 text-[#2c3b4d]" />
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </section>

      {/* Principles Section */}
      <section id="principles" className="py-20 px-4 sm:px-6 lg:px-8 bg-white">
        <div className="max-w-7xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-4xl md:text-5xl font-bold text-[#1C1C1E] mb-6">
              Core Principles
            </h2>
            <p className="text-xl text-[#1C1C1E]/80 max-w-3xl mx-auto">
              Built on seven fundamental principles that ensure security, privacy, and universal adoption.
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
            <Card className="bg-gradient-to-br from-[#2c3b4d]/10 to-[#2c3b4d]/5 border-[#2c3b4d]/20 hover:shadow-lg transition">
              <CardHeader>
                <div className="bg-[#2c3b4d] text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <span className="text-xl font-bold">1</span>
                </div>
                <CardTitle>Small Load</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  {`Minimal processing overhead on devices. The protocol is lightweight and won't impact 
                  phone performance or battery life.`}
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#2c3b4d]/10 to-[#a35139]/5 border-[#2c3b4d]/20 hover:shadow-lg transition">
              <CardHeader>
                <div className="bg-[#2c3b4d] text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <span className="text-xl font-bold">2</span>
                </div>
                <CardTitle>Confidentiality</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Zero-knowledge verification. Impossible to determine who recorded the video or its contents 
                  from the protocol alone.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#a35139]/10 to-[#2c3b4d]/5 border-[#a35139]/20 hover:shadow-lg transition">
              <CardHeader>
                <div className="bg-[#a35139] text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <span className="text-xl font-bold">3</span>
                </div>
                <CardTitle>Encapsulation</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Unique cryptographic hash permanently linked to the video, surviving downloads, 
                  shares, and platform transfers.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#a35139]/10 to-[#a35139]/5 border-[#a35139]/20 hover:shadow-lg transition">
              <CardHeader>
                <div className="bg-[#a35139] text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <span className="text-xl font-bold">4</span>
                </div>
                <CardTitle>Generalized</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Cross-platform implementation across devices and social media platforms. Starting with 
                  iPhone ↔ Instagram integration.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#2c3b4d]/10 to-[#a35139]/5 border-[#2c3b4d]/20 hover:shadow-lg transition">
              <CardHeader>
                <div className="bg-[#2c3b4d] text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <span className="text-xl font-bold">5</span>
                </div>
                <CardTitle>{`Master's Approval`}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Centralized governance body that negotiates partnerships with device manufacturers 
                  and platforms, similar to Apple Pay + Visa.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#a35139]/10 to-[#2c3b4d]/5 border-[#a35139]/20 hover:shadow-lg transition">
              <CardHeader>
                <div className="bg-[#a35139] text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                  <span className="text-xl font-bold">6</span>
                </div>
                <CardTitle>Categorization</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-[#1C1C1E]/80">
                  Different content types (raw, edited) with clear rules. Native edits allowed 
                  (color, cuts), 3D transformations excluded.
                </p>
              </CardContent>
            </Card>

            <Card className="bg-gradient-to-br from-[#2c3b4d]/10 to-[#a35139]/10 border-[#2c3b4d]/20 hover:shadow-lg transition md:col-span-2 lg:col-span-3">
              <CardContent className="pt-6">
                <div className="flex items-start gap-6">
                  <div className="bg-[#2c3b4d] text-white w-12 h-12 rounded-full flex items-center justify-center shrink-0">
                    <span className="text-xl font-bold">7</span>
                  </div>
                  <div>
                    <h3 className="text-xl font-bold text-[#1C1C1E] mb-3">Unbreakable</h3>
                    <p className="text-[#1C1C1E]/80">
                      Built on distributed ledger technology (DLT) and blockchain, ensuring the protocol cannot be 
                      compromised, forged, or circumvented by bad actors. The immutable nature of DLT provides 
                      a tamper-proof foundation for Authentication of Media via Provenance (AMP) verification.
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section id="contact" className="py-20 px-4 sm:px-6 lg:px-8 bg-gradient-to-br from-[#2c3b4d] to-[#1C1C1E]">
        <div className="max-w-4xl mx-auto text-center">
          <h2 className="text-4xl md:text-5xl font-bold text-white mb-6">
            Join the Authenticity Revolution
          </h2>
          <p className="text-xl text-white/80 mb-12">
            Be part of the solution. Get early access and help us build a trustworthy digital future.
          </p>

          <div className="flex flex-col sm:flex-row gap-4 justify-center max-w-2xl mx-auto mb-12">
            <Button size="lg" variant="secondary">
              Get Early Access
            </Button>
            <Button size="lg" variant="outline" className="bg-white/10 text-white border-white/20 hover:bg-white/20">
              Learn More
            </Button>
          </div>

          <div className="mt-12 flex flex-wrap justify-center gap-8 text-white/80">
            <div className="flex items-center gap-2">
              <Smartphone className="h-5 w-5" />
              <span>Device Partners</span>
            </div>
            <div className="flex items-center gap-2">
              <Globe className="h-5 w-5" />
              <span>Platform Integration</span>
            </div>
            <div className="flex items-center gap-2">
              <Logo size={20} />
              <span>Verified Security</span>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="bg-[#1C1C1E] text-white/60 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="grid md:grid-cols-4 gap-8 mb-8">
            <div>
              <div className="flex items-center space-x-2 mb-4">
                <Logo size={24} />
                <span className="text-xl font-bold text-white">DeltaSync</span>
              </div>
              <p className="text-sm">
                Securing authenticity in the age of AI-generated content.
              </p>
            </div>
            
            <div>
              <h4 className="text-white font-semibold mb-4">Product</h4>
              <ul className="space-y-2 text-sm">
                <li><a href="#" className="hover:text-white transition">Features</a></li>
                <li><a href="#" className="hover:text-white transition">How it Works</a></li>
                <li><a href="#" className="hover:text-white transition">Pricing</a></li>
                <li><a href="#" className="hover:text-white transition">Documentation</a></li>
              </ul>
            </div>
            
            <div>
              <h4 className="text-white font-semibold mb-4">Company</h4>
              <ul className="space-y-2 text-sm">
                <li><a href="#" className="hover:text-white transition">About</a></li>
                <li><a href="#" className="hover:text-white transition">Blog</a></li>
                <li><a href="#" className="hover:text-white transition">Careers</a></li>
                <li><a href="#" className="hover:text-white transition">Press</a></li>
              </ul>
            </div>
            
            <div>
              <h4 className="text-white font-semibold mb-4">Legal</h4>
              <ul className="space-y-2 text-sm">
                <li><a href="#" className="hover:text-white transition">Privacy</a></li>
                <li><a href="#" className="hover:text-white transition">Terms</a></li>
                <li><a href="#" className="hover:text-white transition">Security</a></li>
                <li><a href="mailto:info@deltasync.cloud" className="hover:text-white transition">Contact</a></li>
              </ul>
            </div>
          </div>
          
          <div className="border-t border-white/10 pt-8 text-center text-sm">
            <p>&copy; 2025 DeltaSync. All rights reserved.</p>
          </div>
        </div>
      </footer>
    </div>
  );
}
