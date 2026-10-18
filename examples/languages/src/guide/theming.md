# Theming

{{#include ../../../../README.md:theming}}

## Example: a custom mdBook theme

This book adds [Everforest](https://github.com/sainnhe/everforest) to mdBook's
theme picker; choose it from the brush icon in the menu bar. It is added with
mdBook's [theme overrides](https://rust-lang.github.io/mdBook/format/theme/index.html),
and mdBook sets the chosen theme's name as a class on `<html>`, so everything
the theme defines is scoped to `.everforest`.

### Template

`theme/index.hbs` is mdBook's own template, written out with
`mdbook init --theme`, with one entry added to the theme list:

```html
<li role="none"><button role="menuitem" class="theme" id="mdbook-theme-everforest">Everforest</button></li>
```

mdBook uses any file in `theme/` named like one of its own in place of it.
Since this one replaces mdBook's template, refresh it from
`mdbook init --theme` when upgrading mdBook.

### Colours

[`theme/everforest.css`](https://github.com/AlexanderGolys/mdbook-tsitter/blob/main/examples/languages/theme/everforest.css)
sets mdBook's interface variables, the same ones the built-in themes set in
`css/variables.css`, and the highlighting colours. For highlighting it is
enough to set the hues; this theme also maps a few roles to Everforest's own
choices:

```css
.everforest {
{{#include ../../theme/everforest.css:highlighting}}
}
```

The same file styles highlight.js classes in the same roles, so blocks left to
mdBook match the tree-sitter ones.

### Code editors

mdBook gives editable blocks its light editor theme for any theme it does not
know; a short `theme/everforest.js` switches them to the dark one.

The stylesheet and the script are registered in `book.toml`:

```toml
[output.html]
additional-css = ["theme/comparison.css", "theme/everforest.css"]
additional-js = ["theme/everforest.js"]
```
