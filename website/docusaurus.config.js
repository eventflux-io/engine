// @ts-check
import { themes as prismThemes } from 'prism-react-renderer';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Eventflux',
  tagline: 'The Rust-native CEP Engine',
  favicon: 'img/favicon.ico',

  url: 'https://eventflux.io',
  baseUrl: '/',

  organizationName: 'eventflux-io',
  projectName: 'engine',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          editUrl: 'https://github.com/eventflux-io/engine/tree/main/website/',
        },
        blog: {
          showReadingTime: true,
          feedOptions: {
            type: ['rss', 'atom'],
            xslt: true,
          },
          editUrl: 'https://github.com/eventflux-io/engine/tree/main/website/',
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/eventflux-social-card.png',
      colorMode: {
        defaultMode: 'dark',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },
      navbar: {
        title: 'Eventflux',
        logo: {
          alt: 'Eventflux Logo',
          src: 'img/logo.png',
          srcDark: 'img/logo.png',
          width: 32,
          height: 32,
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'tutorialSidebar',
            position: 'left',
            label: 'Docs',
          },
          {
            to: '/docs/rust-api/getting-started',
            label: 'API',
            position: 'left',
          },
          { to: '/blog', label: 'Blog', position: 'left' },
          {
            href: 'https://github.com/eventflux-io/engine',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Documentation',
            items: [
              {
                label: 'Introduction',
                to: '/docs/intro',
              },
              {
                label: 'Quick Start',
                to: '/docs/getting-started/quick-start',
              },
              {
                label: 'SQL Reference',
                to: '/docs/sql-reference/queries',
              },
              {
                label: 'Windows',
                to: '/docs/sql-reference/windows',
              },
            ],
          },
          {
            title: 'Community',
            items: [
              {
                label: 'GitHub Discussions',
                href: 'https://github.com/eventflux-io/engine/discussions',
              },
              {
                label: 'Discord',
                href: 'https://discord.gg/eventflux',
              },
              {
                label: 'Twitter',
                href: 'https://twitter.com/eventflux_io',
              },
            ],
          },
          {
            title: 'More',
            items: [
              {
                label: 'Blog',
                to: '/blog',
              },
              {
                label: 'GitHub',
                href: 'https://github.com/eventflux-io/engine',
              },
              {
                label: 'Architecture',
                to: '/docs/architecture/overview',
              },
              {
                label: 'Rust API',
                to: '/docs/rust-api/getting-started',
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Eventflux Project.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['rust', 'sql', 'toml', 'bash'],
      },
      algolia: {
        // Placeholder - configure when you have Algolia credentials
        appId: 'YOUR_APP_ID',
        apiKey: 'YOUR_SEARCH_API_KEY',
        indexName: 'eventflux',
        contextualSearch: true,
      },
    }),
};

export default config;
