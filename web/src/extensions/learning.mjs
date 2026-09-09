/** Project-local Starlight extension: shared teaching styles and article navigation. */
export default function learningExtensions() {
  return {
    name: 'smashdotnes-learning',
    hooks: {
      'config:setup'({ config, updateConfig }) {
        updateConfig({
          customCss: [...(config.customCss ?? []), './src/styles/docs.css'],
          components: {
            ...config.components,
            Header: './src/components/docs/Header.astro',
            PageTitle: './src/components/docs/PageTitle.astro',
            MobileMenuFooter: './src/components/docs/MobileMenuFooter.astro',
            Footer: './src/components/docs/ArticleFooter.astro',
            ThemeProvider: './src/components/docs/ThemeProvider.astro',
            ThemeSelect: './src/components/docs/ThemeSelect.astro',
          },
        });
      },
    },
  };
}
