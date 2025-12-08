// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  tutorialSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting Started',
      collapsed: false,
      items: [
        'getting-started/installation',
        'getting-started/quick-start',
      ],
    },
    {
      type: 'category',
      label: 'SQL Reference',
      collapsed: false,
      items: [
        'sql-reference/queries',
        'sql-reference/windows',
        'sql-reference/aggregations',
        'sql-reference/joins',
        'sql-reference/patterns',
        'sql-reference/functions',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      collapsed: true,
      items: [
        'architecture/overview',
        'architecture/event-pipeline',
        'architecture/state-management',
      ],
    },
    {
      type: 'category',
      label: 'Rust API',
      collapsed: true,
      items: [
        'rust-api/getting-started',
        'rust-api/configuration',
        'rust-api/testing',
      ],
    },
  ],
};

export default sidebars;
