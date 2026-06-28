# 2con UI Design Guidelines

To maintain proper alignment, cross-platform visual consistency, and recoloring support, all UI components must adhere to the following design conventions.

## The "No Emoji" Convention

> [!IMPORTANT]
> Do NOT use Unicode emoji characters (`🏠`, `⚡`, `⚙`, `🗑`, etc.) inside Slint layout code or text elements.

### Rationale
1. **Theming & Recoloring**: Emojis cannot be dynamically colorized to match active nav highlights or dark/light modes.
2. **Alignment & Sizing**: Emojis render differently across operating systems (Windows Segoe UI Emoji vs. macOS Apple Color Emoji vs. Linux Noto Color Emoji), leading to misaligned layouts and different icon shapes.
3. **Professional Aesthetics**: Using Google Material vector SVGs creates a premium, unified line-art experience.

## Sourcing and Using Icons

1. **Adding Icons**: All vector icons must be sourced from the Google Material Icons set and placed in `ui/images/` as `.svg` files with outline paths.
2. **Registering Icons**: Expose new images in the `Icons` global singleton inside [icons.slint](file:///c:/Users/W/Documents/GitHub/2con/ui/components/icons.slint).
3. **Usage**:
   - Use the `Icon` component for standalone visual tags:
     ```slint
     Icon {
         icon-source: Icons.nav-home;
         icon-color: Theme.accent;
         size: 24px;
     }
     ```
   - Use the `IconButton` component for click triggers:
     ```slint
     IconButton {
         icon-source: Icons.delete;
         text: "Delete";
         background-color: Theme.danger;
         clicked => { /* action */ }
     }
     ```

## Code Reviewer Checklist
- [ ] No emoji unicode codes exist in `.slint` files.
- [ ] All elements are positioned relative to layout rows/columns. No absolute manual `x`/`y` centering offsets exist.
- [ ] Colors and spacing parameters utilize named tokens from `Theme`.
