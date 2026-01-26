# ZenUML Web Renderer Deployment

This document describes the deployment process for the ZenUML Web Renderer to Cloudflare Pages.

## Deployment Strategy

- **Production**: Deploys from `main` branch to `zenuml-web-renderer` project
- **Staging**: Deploys from any non-main branch to `zenuml-web-renderer-staging` project

## GitHub Actions Workflow

The deployment is automated through GitHub Actions (`.github/workflows/cloudflare-pages.yml`):

1. **Triggers**: 
   - Push to `main` or `feat/public-renderer` branches
   - Pull requests to `main` branch
   - Only when relevant files are changed (renderer.html, public/**, src/**, etc.)

2. **Build Process**:
   - Install dependencies with pnpm
   - Build the site using `pnpm build:site`
   - Deploy to appropriate Cloudflare Pages project

## Required Secrets

Add these secrets to your GitHub repository settings:

- `CLOUDFLARE_API_TOKEN`: Cloudflare API token with Pages permissions
- `CLOUDFLARE_ACCOUNT_ID`: Your Cloudflare account ID

## Manual Deployment

You can also deploy manually using wrangler:

```bash
# Deploy to staging
pnpm pages:deploy:staging

# Deploy to production  
pnpm pages:deploy
```

## Project Structure

- `renderer.html`: Main renderer page (root level)
- `public/renderer.html`: Copy of renderer page for static hosting
- `dist/`: Build output directory (created by `pnpm build:site`)
- `wrangler.toml`: Cloudflare configuration

## URLs

- **Production**: `https://zenuml-web-renderer.pages.dev`
- **Staging**: `https://zenuml-web-renderer-staging.pages.dev`

## Usage

Once deployed, you can use the renderer by visiting:
- `https://your-domain.pages.dev/renderer.html`
- `https://your-domain.pages.dev/` (if configured as index)

Future URL parameter support will allow:
- `https://your-domain.pages.dev/renderer.html?code=<base64-encoded-zenuml>`