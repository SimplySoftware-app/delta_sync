'use client';

import { Shield, Lock, Layers, Globe, CheckCircle, AlertTriangle, Video, Smartphone } from 'lucide-react';
import { useState } from 'react';

export default function Home() {
  const [email, setEmail] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Handle email submission
    alert('Thank you for your interest! We\'ll keep you updated.');
    setEmail('');
  };

  return (
    <div className="min-h-screen bg-linear-to-br from-slate-50 via-blue-50 to-indigo-50">
      {/* Navigation */}
      <nav className="fixed top-0 w-full bg-white/80 backdrop-blur-md z-50 border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center space-x-2">
              <Shield className="h-8 w-8 text-blue-600" />
              <span className="text-2xl font-bold bg-linear-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent">
                DeltaSync
              </span>
            </div>
            <div className="hidden md:flex space-x-8">
              <a href="#problem" className="text-gray-700 hover:text-blue-600 transition">Problem</a>
              <a href="#solution" className="text-gray-700 hover:text-blue-600 transition">Solution</a>
              <a href="#principles" className="text-gray-700 hover:text-blue-600 transition">Principles</a>
              <a href="#contact" className="text-gray-700 hover:text-blue-600 transition">Contact</a>
            </div>
            <button className="bg-blue-600 text-white px-6 py-2 rounded-full hover:bg-blue-700 transition">
              Get Started
            </button>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="pt-32 pb-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="text-center animate-fade-in">
            <div className="inline-flex items-center space-x-2 bg-blue-100 text-blue-700 px-4 py-2 rounded-full mb-6">
              <Video className="h-4 w-4" />
              <span className="text-sm font-medium">Revolutionizing Content Authenticity</span>
            </div>
            <h1 className="text-5xl md:text-7xl font-bold text-gray-900 mb-6 leading-tight">
              Authentic Content in the
              <span className="block bg-linear-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent">
                Age of AI
              </span>
            </h1>
            <p className="text-xl md:text-2xl text-gray-600 mb-12 max-w-3xl mx-auto">
              A revolutionary protocol that ensures video authenticity through cryptographic verification, 
              protecting viewers from AI-generated deception.
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <button className="bg-blue-600 text-white px-8 py-4 rounded-full text-lg font-semibold hover:bg-blue-700 transition shadow-lg hover:shadow-xl transform hover:-translate-y-1">
                Join the Waitlist
              </button>
              <button className="bg-white text-blue-600 px-8 py-4 rounded-full text-lg font-semibold hover:bg-gray-50 transition border-2 border-blue-600">
                Learn More
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* Problem Section */}
      <section id="problem" className="py-20 px-4 sm:px-6 lg:px-8 bg-white">
        <div className="max-w-7xl mx-auto">
          <div className="text-center mb-16">
            <div className="inline-flex items-center space-x-2 bg-red-100 text-red-700 px-4 py-2 rounded-full mb-4">
              <AlertTriangle className="h-4 w-4" />
              <span className="text-sm font-medium">The Growing Crisis</span>
            </div>
            <h2 className="text-4xl md:text-5xl font-bold text-gray-900 mb-6">
              The AI Content Problem
            </h2>
            <p className="text-xl text-gray-600 max-w-3xl mx-auto">
              We're living in an era where AI-generated content is flooding the internet, 
              exploiting our dopamine systems and eroding trust in digital media.
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8 mb-12">
            <div className="bg-linear-to-br from-red-50 to-orange-50 p-8 rounded-2xl">
              <div className="bg-red-100 w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <Video className="h-6 w-6 text-red-600" />
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Fake Engagement</h3>
              <p className="text-gray-600">
                Content creators are generating fake videos to manipulate engagement and monetize attention 
                through deceptive means.
              </p>
            </div>

            <div className="bg-linear-to-br from-orange-50 to-yellow-50 p-8 rounded-2xl">
              <div className="bg-orange-100 w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <AlertTriangle className="h-6 w-6 text-orange-600" />
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Trust Erosion</h3>
              <p className="text-gray-600">
                As AI becomes more sophisticated, distinguishing real from fake becomes impossible, 
                leading to widespread distrust.
              </p>
            </div>

            <div className="bg-linear-to-br from-yellow-50 to-red-50 p-8 rounded-2xl">
              <div className="bg-yellow-100 w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <Globe className="h-6 w-6 text-yellow-600" />
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Platform Exodus</h3>
              <p className="text-gray-600">
                Without solutions, users will abandon platforms entirely, and authentic creators 
                will lose their audiences.
              </p>
            </div>
          </div>

          <div className="bg-linear-to-r from-gray-900 to-gray-800 p-8 rounded-2xl text-white">
            <p className="text-xl md:text-2xl font-semibold text-center">
              "By 2026, if we don't act now, authentic content will be indistinguishable from AI-generated media, 
              causing a mass exodus from social platforms."
            </p>
          </div>
        </div>
      </section>

      {/* Solution Section */}
      <section id="solution" className="py-20 px-4 sm:px-6 lg:px-8 bg-linear-to-br from-blue-50 to-indigo-50">
        <div className="max-w-7xl mx-auto">
          <div className="text-center mb-16">
            <div className="inline-flex items-center space-x-2 bg-green-100 text-green-700 px-4 py-2 rounded-full mb-4">
              <CheckCircle className="h-4 w-4" />
              <span className="text-sm font-medium">Our Solution</span>
            </div>
            <h2 className="text-4xl md:text-5xl font-bold text-gray-900 mb-6">
              Introducing DeltaSync Protocol
            </h2>
            <p className="text-xl text-gray-600 max-w-3xl mx-auto">
              A cryptographic protocol that creates an unbreakable chain of authenticity from device 
              to platform, ensuring every video can be verified as genuine.
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-12 items-center">
            <div>
              <h3 className="text-3xl font-bold text-gray-900 mb-6">How It Works</h3>
              <div className="space-y-6">
                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-blue-600 text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      1
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-gray-900 mb-2">Device Capture</h4>
                    <p className="text-gray-600">
                      Videos are cryptographically signed at the moment of capture by trusted devices 
                      (iPhone, Pixel, Samsung, etc.)
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-blue-600 text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      2
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-gray-900 mb-2">Unique Fingerprint</h4>
                    <p className="text-gray-600">
                      Each video receives a unique cryptographic hash that remains attached through downloads 
                      and shares.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-blue-600 text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      3
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-gray-900 mb-2">Platform Verification</h4>
                    <p className="text-gray-600">
                      Social platforms can instantly verify authenticity and label content accordingly, 
                      giving users confidence.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="shrink-0">
                    <div className="bg-blue-600 text-white w-8 h-8 rounded-full flex items-center justify-center font-bold">
                      4
                    </div>
                  </div>
                  <div>
                    <h4 className="font-bold text-gray-900 mb-2">User Control</h4>
                    <p className="text-gray-600">
                      Users can filter their feeds to show only verified authentic content, eliminating 
                      AI-generated spam.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-white p-8 rounded-2xl shadow-xl">
              <div className="aspect-square bg-linear-to-br from-blue-100 to-indigo-100 rounded-xl flex items-center justify-center">
                <Shield className="h-32 w-32 text-blue-600" />
              </div>
              <div className="mt-6 space-y-4">
                <div className="flex items-center justify-between p-4 bg-green-50 rounded-lg">
                  <span className="font-semibold text-gray-900">Authenticity</span>
                  <CheckCircle className="h-6 w-6 text-green-600" />
                </div>
                <div className="flex items-center justify-between p-4 bg-blue-50 rounded-lg">
                  <span className="font-semibold text-gray-900">Privacy Protected</span>
                  <Lock className="h-6 w-6 text-blue-600" />
                </div>
                <div className="flex items-center justify-between p-4 bg-purple-50 rounded-lg">
                  <span className="font-semibold text-gray-900">Blockchain Verified</span>
                  <Layers className="h-6 w-6 text-purple-600" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Principles Section */}
      <section id="principles" className="py-20 px-4 sm:px-6 lg:px-8 bg-white">
        <div className="max-w-7xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-4xl md:text-5xl font-bold text-gray-900 mb-6">
              Core Principles
            </h2>
            <p className="text-xl text-gray-600 max-w-3xl mx-auto">
              Built on seven fundamental principles that ensure security, privacy, and universal adoption.
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
            <div className="bg-linear-to-br from-blue-50 to-indigo-50 p-8 rounded-2xl hover:shadow-lg transition">
              <div className="bg-blue-600 text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <span className="text-xl font-bold">1</span>
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Small Load</h3>
              <p className="text-gray-600">
                Minimal processing overhead on devices. The protocol is lightweight and won't impact 
                phone performance or battery life.
              </p>
            </div>

            <div className="bg-linear-to-br from-purple-50 to-pink-50 p-8 rounded-2xl hover:shadow-lg transition">
              <div className="bg-purple-600 text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <span className="text-xl font-bold">2</span>
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Confidentiality</h3>
              <p className="text-gray-600">
                Zero-knowledge verification. Impossible to determine who recorded the video or its contents 
                from the protocol alone.
              </p>
            </div>

            <div className="bg-linear-to-br from-green-50 to-teal-50 p-8 rounded-2xl hover:shadow-lg transition">
              <div className="bg-green-600 text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <span className="text-xl font-bold">3</span>
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Encapsulation</h3>
              <p className="text-gray-600">
                Unique cryptographic hash permanently linked to the video, surviving downloads, 
                shares, and platform transfers.
              </p>
            </div>

            <div className="bg-linear-to-br from-orange-50 to-red-50 p-8 rounded-2xl hover:shadow-lg transition">
              <div className="bg-orange-600 text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <span className="text-xl font-bold">4</span>
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Generalized</h3>
              <p className="text-gray-600">
                Cross-platform implementation across devices and social media platforms. Starting with 
                iPhone ↔ Instagram integration.
              </p>
            </div>

            <div className="bg-linear-to-br from-yellow-50 to-orange-50 p-8 rounded-2xl hover:shadow-lg transition">
              <div className="bg-yellow-600 text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <span className="text-xl font-bold">5</span>
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Master's Approval</h3>
              <p className="text-gray-600">
                Centralized governance body that negotiates partnerships with device manufacturers 
                and platforms, similar to Apple Pay + Visa.
              </p>
            </div>

            <div className="bg-linear-to-br from-pink-50 to-purple-50 p-8 rounded-2xl hover:shadow-lg transition">
              <div className="bg-pink-600 text-white w-12 h-12 rounded-full flex items-center justify-center mb-4">
                <span className="text-xl font-bold">6</span>
              </div>
              <h3 className="text-xl font-bold text-gray-900 mb-3">Categorization</h3>
              <p className="text-gray-600">
                Different content types (raw, edited) with clear rules. Native edits allowed 
                (color, cuts), 3D transformations excluded.
              </p>
            </div>

            <div className="bg-linear-to-br from-indigo-50 to-blue-50 p-8 rounded-2xl hover:shadow-lg transition md:col-span-2 lg:col-span-3">
              <div className="flex items-start gap-6">
                <div className="bg-indigo-600 text-white w-12 h-12 rounded-full flex items-center justify-center shrink-0">
                  <span className="text-xl font-bold">7</span>
                </div>
                <div>
                  <h3 className="text-xl font-bold text-gray-900 mb-3">Unbreakable</h3>
                  <p className="text-gray-600">
                    Built on blockchain or similar immutable technology, ensuring the protocol cannot be 
                    compromised, forged, or circumvented by bad actors.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section id="contact" className="py-20 px-4 sm:px-6 lg:px-8 bg-linear-to-br from-blue-600 to-indigo-700">
        <div className="max-w-4xl mx-auto text-center">
          <h2 className="text-4xl md:text-5xl font-bold text-white mb-6">
            Join the Authenticity Revolution
          </h2>
          <p className="text-xl text-blue-100 mb-12">
            Be part of the solution. Get early access and help us build a trustworthy digital future.
          </p>
          
          <form onSubmit={handleSubmit} className="flex flex-col sm:flex-row gap-4 max-w-2xl mx-auto">
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="Enter your email"
              required
              className="flex-1 px-6 py-4 rounded-full text-gray-900 text-lg focus:outline-none focus:ring-4 focus:ring-blue-300"
            />
            <button
              type="submit"
              className="bg-white text-blue-600 px-8 py-4 rounded-full text-lg font-semibold hover:bg-gray-100 transition shadow-lg"
            >
              Get Early Access
            </button>
          </form>

          <div className="mt-12 flex flex-wrap justify-center gap-8 text-blue-100">
            <div className="flex items-center gap-2">
              <Smartphone className="h-5 w-5" />
              <span>Device Partners</span>
            </div>
            <div className="flex items-center gap-2">
              <Globe className="h-5 w-5" />
              <span>Platform Integration</span>
            </div>
            <div className="flex items-center gap-2">
              <Shield className="h-5 w-5" />
              <span>Verified Security</span>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="bg-gray-900 text-gray-400 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="grid md:grid-cols-4 gap-8 mb-8">
            <div>
              <div className="flex items-center space-x-2 mb-4">
                <Shield className="h-6 w-6 text-blue-500" />
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
                <li><a href="#" className="hover:text-white transition">Contact</a></li>
              </ul>
            </div>
          </div>
          
          <div className="border-t border-gray-800 pt-8 text-center text-sm">
            <p>&copy; 2025 DeltaSync. All rights reserved.</p>
          </div>
        </div>
      </footer>
    </div>
  );
}
