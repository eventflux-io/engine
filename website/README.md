# Eventflux Documentation Website

This directory contains the Docusaurus v3 documentation and marketing website for Eventflux.

## Prerequisites

- **Node.js** >= 18.0
- **npm** >= 9.0

## Quick Start

```bash
# From repository root, navigate to website directory
cd website

# Install dependencies
npm install

# Start development server
npm start
```

The site will be available at **http://localhost:3000**

## Available Commands

| Command | Description |
|---------|-------------|
| `npm start` | Start dev server with hot reload |
| `npm run build` | Build production site to `build/` |
| `npm run serve` | Serve production build locally |
| `npm run clear` | Clear Docusaurus cache |

## Building for Production

```bash
# Build optimized production bundle
npm run build

# Preview the production build
npm run serve
```

Build output will be in the `build/` directory.

## Deployment

Deployment is automated via GitHub Actions:

- **Trigger**: Push to `main` branch with changes in `website/`
- **Target**: GitHub Pages
- **Workflow**: `.github/workflows/deploy-docs.yml`

### Manual Deployment

```bash
# Build and deploy to GitHub Pages
npm run deploy
```

## Directory Structure

```
website/
├── blog/                  # Blog posts (MDX)
│   ├── authors.yml        # Blog author definitions
│   └── YYYY-MM-DD-*.md    # Blog posts
├── docs/                  # Documentation (MDX)
│   ├── intro.md           # Getting started
│   ├── architecture/      # Architecture docs
│   ├── sql-reference/     # SQL language reference
│   └── rust-api/          # Rust API documentation
├── src/
│   ├── components/        # React components
│   │   └── ScenarioBlock/ # Scrollytelling component
│   ├── css/
│   │   └── custom.css     # Global custom styles
│   └── pages/
│       ├── index.js       # Landing page
│       └── index.module.css
├── static/
│   └── img/               # Static images
├── docusaurus.config.js   # Site configuration
├── sidebars.js            # Sidebar navigation
├── babel.config.js        # Babel configuration
└── package.json
```

## Writing Documentation

### Creating a New Page

1. Create a new `.md` or `.mdx` file in `docs/`
2. Add frontmatter:
   ```yaml
   ---
   sidebar_position: 1
   title: Page Title
   description: Brief description for SEO
   ---
   ```
3. Update `sidebars.js` if adding a new section

### MDX Features

Docusaurus supports MDX (Markdown + JSX). Available components:

```mdx
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

<Tabs>
  <TabItem value="rust" label="Rust" default>
    Rust code here
  </TabItem>
  <TabItem value="sql" label="SQL">
    SQL code here
  </TabItem>
</Tabs>
```

**Admonitions:**

```md
:::tip Title
Helpful tip content
:::

:::warning
Warning content
:::

:::info
Informational note
:::
```

### Common Gotchas

- **Escape `<` in tables**: Use `\<` to avoid MDX parsing as JSX
- **Code blocks**: Use triple backticks with language identifier
- **Links**: Use `/docs/path` for internal links (not relative paths)

## Writing Blog Posts

1. Create file: `blog/YYYY-MM-DD-slug.md`
2. Add frontmatter:
   ```yaml
   ---
   slug: my-post
   title: Post Title
   authors: [eventflux]
   tags: [announcement, release]
   ---
   ```
3. Use `<!-- truncate -->` to mark excerpt boundary

## Customization

### Styling

Edit `src/css/custom.css` for global styles. The site uses:
- **Dark mode** as default
- **Infima** CSS framework (Docusaurus default)
- CSS Modules for component-specific styles

### Configuration

Key settings in `docusaurus.config.js`:
- `title`, `tagline`, `url` - Site metadata
- `navbar` - Navigation links
- `footer` - Footer links and content
- `prism` - Code syntax highlighting languages

## Troubleshooting

**Cache issues:**
```bash
npm run clear && npm start
```

**Port already in use:**
```bash
npm start -- --port 3001
```

**Build errors:**
Check that all MDX files have valid syntax (especially `<` characters in tables).
