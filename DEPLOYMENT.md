# DeltaSync Deployment Guide

Production for the landing is self-hosted. See the root `Dockerfile`, `docker-compose.yml`, and `.github/workflows/deploy.yml`. `vercel.json` is unchanged and is not used for `deltasync.simplysoftware.app`. The steps below are the previous Vercel setup.

## Quick Start

The project is ready for deployment! Follow these steps:

## 1. Test Locally

First, test the site locally to ensure everything works:

```bash
cd /Users/carlosedm10/projects/delta_sync
npm run dev
```

Open http://localhost:3000 in your browser.

## 2. Deploy to Vercel

### Option A: Using Vercel CLI (Recommended)

1. Install Vercel CLI globally:
```bash
npm i -g vercel
```

2. Login to Vercel:
```bash
vercel login
```

3. Deploy:
```bash
cd /Users/carlosedm10/projects/delta_sync
vercel
```

4. For production:
```bash
vercel --prod
```

### Option B: Using GitHub + Vercel Dashboard

1. **Push to GitHub:**
   ```bash
   cd /Users/carlosedm10/projects/delta_sync
   git init
   git add .
   git commit -m "Initial commit: DeltaSync landing page"
   git branch -M main
   git remote add origin <your-github-repo-url>
   git push -u origin main
   ```

2. **Import to Vercel:**
   - Go to https://vercel.com/new
   - Import your GitHub repository
   - Vercel will auto-detect Next.js settings
   - Click "Deploy"

## 3. Configure Custom Domain (deltasync.simplysoftware.app)

### In Vercel:

1. Go to your project in Vercel dashboard
2. Click "Settings" → "Domains"
3. Add domain: `deltasync.simplysoftware.app`
4. Add domain: `www.deltasync.simplysoftware.app`
5. Vercel will provide DNS records to configure

### In GoDaddy:

1. Log in to your GoDaddy account
2. Go to "My Products" → "DNS" for deltasync.simplysoftware.app
3. Click "Manage DNS"

4. **Add these DNS records:**

   **For root domain (@):**
   - Type: `A`
   - Name: `@`
   - Value: `76.76.21.21`
   - TTL: `600` (or default)

   **For www subdomain:**
   - Type: `CNAME`
   - Name: `www`
   - Value: `cname.vercel-dns.com`
   - TTL: `600` (or default)

5. **Save all records**

6. Wait 24-48 hours for DNS propagation (usually much faster, ~10 minutes)

### Verify SSL Certificate

Vercel automatically provisions SSL certificates. After DNS propagation:
- Visit https://deltasync.simplysoftware.app
- Visit https://www.deltasync.simplysoftware.app
- Both should work with valid SSL

## 4. Environment Variables (if needed)

If you add any API keys or secrets later:

1. In Vercel dashboard: Settings → Environment Variables
2. Add variables for:
   - Development
   - Preview
   - Production

3. Redeploy for changes to take effect

## 5. Custom Configuration

The project includes:
- ✅ `vercel.json` - Vercel configuration
- ✅ `.vercelignore` - Files to ignore during deployment
- ✅ Optimized Next.js config
- ✅ SEO metadata configured
- ✅ Responsive design
- ✅ Modern UI with Tailwind CSS

## Troubleshooting

### Build Fails
```bash
# Test build locally first
npm run build
```

### DNS Not Resolving
- Use `dig deltasync.simplysoftware.app` to check DNS records
- Wait longer for propagation
- Verify records in GoDaddy match Vercel's requirements

### SSL Issues
- Vercel handles SSL automatically
- May take a few minutes after DNS propagation
- Check Vercel dashboard for certificate status

## Post-Deployment Checklist

- [ ] Test all navigation links
- [ ] Verify responsive design on mobile/tablet
- [ ] Check page load speed (should be < 2s)
- [ ] Test email form submission
- [ ] Verify SEO metadata in browser inspector
- [ ] Check social media preview (OpenGraph)
- [ ] Test all CTAs and buttons

## Performance Monitoring

Vercel provides:
- Analytics dashboard
- Core Web Vitals
- Real user monitoring
- Deployment logs

Access via: https://vercel.com/dashboard

## Support

- Vercel Docs: https://vercel.com/docs
- Next.js Docs: https://nextjs.org/docs
- GoDaddy Support: https://www.godaddy.com/help

---

Your DeltaSync website is production-ready! 🚀

