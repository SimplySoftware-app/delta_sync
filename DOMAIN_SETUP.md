# Connecting GoDaddy Domain to Vercel - Step by Step

The landing is served from Hetzner at `deltasync.simplysoftware.app`, not from Vercel. `vercel.json` is still in the repo. This document is the previous GoDaddy-to-Vercel setup.

## Prerequisites
- Your project must be deployed on Vercel first
- You need access to your GoDaddy account

---

## Step 1: Add Domain in Vercel Dashboard

1. **Go to Vercel Dashboard**
   - Visit https://vercel.com/dashboard
   - Click on your **DeltaSync project**

2. **Navigate to Domain Settings**
   - Click on the **"Settings"** tab (top navigation)
   - Click on **"Domains"** in the left sidebar

3. **Add Your Domain**
   - In the "Domains" section, you'll see an input field
   - Type: `deltasync.simplysoftware.app`
   - Press Enter or click "Add"
   - Vercel will show you the DNS records you need to configure

4. **Add WWW Subdomain (Optional but Recommended)**
   - Also add: `www.deltasync.simplysoftware.app`
   - This ensures both `deltasync.simplysoftware.app` and `www.deltasync.simplysoftware.app` work

5. **Copy the DNS Records**
   - Vercel will display specific DNS records you need to add
   - **IMPORTANT**: Use the exact values Vercel provides, not generic ones
   - You'll typically see:
     - An **A record** for the root domain (@)
     - A **CNAME record** for www subdomain

---

## Step 2: Configure DNS in GoDaddy

### Access DNS Management

1. **Log in to GoDaddy**
   - Go to https://www.godaddy.com
   - Sign in to your account

2. **Navigate to DNS Settings**
   - Click on **"My Products"** (top right)
   - Find **deltasync.simplysoftware.app** in your domain list
   - Click on the **"DNS"** button (or three dots → "Manage DNS")

3. **Edit DNS Records**

   You'll see a list of existing DNS records. You need to:

   **Option A: If Vercel gave you an A record for root domain:**
   
   - Find or create an **A record** with:
     - **Type**: `A`
     - **Name**: `@` (or leave blank, or `deltasync.simplysoftware.app`)
     - **Value**: The IP address Vercel provided (usually starts with `76.` or `151.`)
     - **TTL**: `600` (or leave default)
   
   - Find or create a **CNAME record** for www:
     - **Type**: `CNAME`
     - **Name**: `www`
     - **Value**: The CNAME value Vercel provided (usually `cname.vercel-dns.com` or similar)
     - **TTL**: `600` (or leave default)

   **Option B: If Vercel gave you a CNAME for root domain (newer method):**
   
   - Some domains support CNAME at root (ALIAS/ANAME record)
   - Follow Vercel's specific instructions
   - Usually: Create a CNAME with Name `@` pointing to Vercel's value

4. **Remove Conflicting Records**
   - Delete any existing A records pointing to other IPs
   - Delete any existing CNAME records for www pointing elsewhere
   - Keep MX records (for email) if you use email with this domain

5. **Save Changes**
   - Click "Save" after each record addition/modification
   - GoDaddy will confirm the changes

---

## Step 3: Wait for DNS Propagation

1. **Propagation Time**
   - DNS changes can take **5 minutes to 48 hours**
   - Usually completes within **10-30 minutes**
   - Vercel will show the status in the dashboard

2. **Check Status in Vercel**
   - Go back to Vercel → Settings → Domains
   - You'll see the status next to your domain:
     - ⏳ **"Pending"** - DNS is propagating
     - ✅ **"Valid Configuration"** - Domain is connected!
     - ❌ **"Invalid Configuration"** - Check DNS records

3. **Verify DNS Propagation**
   - Use online tools to check:
     - https://dnschecker.org
     - Search for `deltasync.simplysoftware.app` A record
     - Should show Vercel's IP address

---

## Step 4: SSL Certificate (Automatic)

- Vercel **automatically provisions SSL certificates**
- Once DNS is configured, SSL will be active within minutes
- Your site will be accessible via `https://deltasync.simplysoftware.app`
- No additional configuration needed!

---

## Troubleshooting

### Domain shows "Invalid Configuration"

1. **Double-check DNS records in GoDaddy:**
   - Ensure A record value matches exactly what Vercel provided
   - Ensure CNAME record value matches exactly
   - Check for typos

2. **Wait longer:**
   - DNS can take up to 48 hours (rare)
   - Clear your browser cache
   - Try incognito/private browsing

3. **Verify in Vercel:**
   - Go to Settings → Domains
   - Click on your domain
   - Vercel will show what it's detecting
   - Compare with what you configured in GoDaddy

### Domain not resolving

1. **Check DNS propagation:**
   ```bash
   # In terminal, check DNS:
   dig deltasync.simplysoftware.app
   # or
   nslookup deltasync.simplysoftware.app
   ```

2. **Verify records in GoDaddy:**
   - Make sure records are saved
   - Check TTL values aren't too high (600 is good)

3. **Contact Support:**
   - Vercel support: https://vercel.com/support
   - GoDaddy support: https://www.godaddy.com/help

---

## Quick Reference: Common Vercel DNS Values

**Note**: Always use the values Vercel provides in your dashboard, but here are common patterns:

- **A Record (Root)**: `76.76.21.21` or `151.101.1.195` (Vercel will tell you)
- **CNAME (www)**: `cname.vercel-dns.com` (Vercel will tell you)

---

## Success Checklist

- [ ] Domain added in Vercel dashboard
- [ ] DNS records configured in GoDaddy
- [ ] DNS records match exactly what Vercel provided
- [ ] Waited for DNS propagation (10-30 min)
- [ ] Domain shows "Valid Configuration" in Vercel
- [ ] Can access https://deltasync.simplysoftware.app
- [ ] SSL certificate is active (green lock in browser)

---

## Need Help?

- **Vercel Domain Docs**: https://vercel.com/docs/concepts/projects/domains
- **GoDaddy DNS Help**: https://www.godaddy.com/help/manage-dns-records-680
- **Vercel Support**: support@vercel.com

