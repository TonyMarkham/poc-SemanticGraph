---
name: Graphify Narrative
colors:
  surface: '#111318'
  surface-dim: '#111318'
  surface-bright: '#37393e'
  surface-container-lowest: '#0c0e12'
  surface-container-low: '#1a1c20'
  surface-container: '#1e2024'
  surface-container-high: '#282a2e'
  surface-container-highest: '#333539'
  on-surface: '#e2e2e8'
  on-surface-variant: '#bac9cc'
  inverse-surface: '#e2e2e8'
  inverse-on-surface: '#2f3035'
  outline: '#849396'
  outline-variant: '#3b494c'
  surface-tint: '#00daf3'
  primary: '#c3f5ff'
  on-primary: '#00363d'
  primary-container: '#00e5ff'
  on-primary-container: '#00626e'
  inverse-primary: '#006875'
  secondary: '#dab9ff'
  on-secondary: '#460283'
  secondary-container: '#602b9d'
  on-secondary-container: '#cfa7ff'
  tertiary: '#95ffef'
  on-tertiary: '#003731'
  tertiary-container: '#30e8d4'
  on-tertiary-container: '#00645a'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#9cf0ff'
  primary-fixed-dim: '#00daf3'
  on-primary-fixed: '#001f24'
  on-primary-fixed-variant: '#004f58'
  secondary-fixed: '#eedbff'
  secondary-fixed-dim: '#dab9ff'
  on-secondary-fixed: '#2a0053'
  on-secondary-fixed-variant: '#5e289b'
  tertiary-fixed: '#4ffbe6'
  tertiary-fixed-dim: '#17deca'
  on-tertiary-fixed: '#00201c'
  on-tertiary-fixed-variant: '#005048'
  background: '#111318'
  on-background: '#e2e2e8'
  surface-variant: '#333539'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 32px
    fontWeight: '600'
    lineHeight: '1.2'
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: '1.3'
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: '1.6'
  code-sm:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: '400'
    lineHeight: '1.5'
  label-caps:
    fontFamily: JetBrains Mono
    fontSize: 10px
    fontWeight: '700'
    lineHeight: '1'
    letterSpacing: 0.1em
  node-label:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '500'
    lineHeight: '1'
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  unit: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 32px
  panel-width: 320px
  toolbar-height: 48px
---

## Brand & Style

This design system is engineered for deep cognitive work, focusing on the synthesis of complex data structures and node-based relationships. The brand personality is **technical, analytical, and precise**, evoking the feeling of a high-end "pro-tool" used for knowledge discovery. 

The aesthetic is a hybrid of **Minimalist-Technical** and **Modern Glassmorphism**. It prioritizes high information density while maintaining clarity through strict hierarchy and structural alignment. The emotional response should be one of "command and control"—providing the user with a sophisticated environment to map, link, and visualize large datasets without visual fatigue.

Key stylistic pillars:
- **Optical Precision:** Every line and border serves a structural purpose.
- **Luminous Hierarchy:** Light is used sparingly to indicate activity or importance against a void-like backdrop.
- **Connectivity:** Visual metaphors of nodes, edges, and clusters are woven into the UI through subtle glow effects and path-like indicators.

## Colors

The palette is built on a "Deep Space" foundation to maximize the contrast of luminous data points. 

### Foundation
- **Base Canvas:** A near-black charcoal (#050608) used for the infinite graph background.
- **Surface:** Deep navy-charcoal with 80% opacity for panels, creating a sense of depth over the graph.

### Accents & Data States
- **Primary (Electric Blue):** Used for active selections, primary actions, and high-confidence connections.
- **Secondary (Neon Purple):** Used for inferred relationships and secondary navigation elements.
- **Tertiary (Mint Green):** Used for verified data "Extracted" states.

### Semantic Nodes
- **Extracted:** Mint Green. Represents high-certainty, ground-truth data.
- **Inferred:** Neon Purple. Represents algorithmically generated or suggested data.
- **Ambiguous:** Warm Amber. Represents data requiring manual resolution or low confidence.

### Clusters
Four distinct vibrant hues (Red, Pink, Blue, Yellow) are reserved for community detection and clustering, ensuring nodes belonging to the same group are instantly recognizable.

## Typography

The system utilizes a dual-font approach to balance readability with technical utility.

1. **Inter (Sans-Serif):** The primary workhorse for all interface text, headings, and node labels. It provides a modern, neutral tone that stays legible at small sizes.
2. **JetBrains Mono (Monospace):** Used for metadata, coordinates, code snippets, and UI labels. The fixed-width nature reinforces the "pro-tool" aesthetic and is used wherever raw data is displayed.

**Scale Philosophy:**
Because this is a data-dense tool, font sizes are generally smaller than consumer apps. We rely on font weight and the "label-caps" style to differentiate levels of hierarchy within sidebars and inspector panels.

## Layout & Spacing

The layout utilizes a **Fixed-Fluid Hybrid** model optimized for high-resolution desktop displays.

### Structure
- **Infinite Canvas:** The central viewport where the node-graph lives.
- **Fixed Sidebars:** Left (Navigation/Project) and Right (Inspector/Detail) panels are fixed at 320px. 
- **Global Header:** A slim 48px bar for global search and utility actions.

### Spacing Rhythm
The system follows a strict **4px grid**. 
- **Internal Padding:** Use 12px or 16px for panel content.
- **Density:** Components like lists and tables should use 4px-8px vertical spacing to maximize data visibility.
- **Gutters:** Standard 1px borders separate panels to maintain a crisp, architectural feel.

## Elevation & Depth

Depth is achieved through **translucency and luminosity** rather than traditional drop shadows.

### Tiers
1. **Level 0 (Canvas):** The deepest layer. Pure black or dark charcoal.
2. **Level 1 (Panels):** Glassmorphic surfaces with a 20px backdrop blur and a subtle 1px inner stroke (`rgba(255,255,255,0.1)`). 
3. **Level 2 (Popovers/Tooltips):** Floating elements with a more opaque background and a "glow" border using the primary color at low opacity.

### Interactions
- **Hover:** Elements should increase in brightness or gain a subtle outer glow (0px 0px 8px).
- **Selection:** Nodes and edges are highlighted with the Primary Electric Blue, increasing the stroke weight and adding a vibrant glow.

## Shapes

The shape language is **geometric and sharp**. We use "Soft" roundedness (4px) to prevent the UI from feeling aggressive while maintaining a professional, engineered look.

- **Nodes:** Circular for entities, Diamond for actions/logic.
- **Panels/Buttons:** 4px radius. 
- **Input Fields:** 2px radius or sharp edges to emphasize the "coding" environment.
- **Edges (Graph):** Straight lines or slightly curved Bezier paths with 1.5px thickness.

## Components

### Buttons
- **Primary:** Solid Electric Blue with black text. No rounded corners (Sharp).
- **Ghost:** Transparent background, 1px Primary border, Primary text.
- **Action Icons:** 24x24px hit area, 16px icon size.

### Nodes (The Core Component)
- **Visuals:** 12px-24px diameter circles.
- **States:** 
  - *Default:* Hollow with colored border.
  - *Hover:* Fill with 20% color opacity + label appears.
  - *Selected:* Solid fill + 4px glow.
- **Labeling:** Text positioned to the right of the node in `node-label` style.

### Inspector Panels (Glassmorphism)
- Sidebars should use `surface_panel` variables with `backdrop-filter: blur(20px)`.
- Use `label-caps` for section headers (e.g., ATTRIBUTES, METRICS).

### Connection Edges
- **Confirmed:** Solid line, 1px.
- **Inferred:** Dashed line (4px dash, 4px gap).
- **Active Path:** 2px Electric Blue with animated "marching ants" effect if data is flowing.

### Chips / Badges
- Used for "Confidence Score" and "Category".
- Design: Monospace font, background matching the state color at 15% opacity, text at 100% opacity.