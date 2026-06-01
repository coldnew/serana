# Directory Structure

> How backend code is organized in this project.

---

## Overview

<!--
Document your project's backend directory structure here.

Questions to answer:
- How are modules/packages organized?
- Where does business logic live?
- Where are API endpoints defined?
- How are utilities and helpers organized?
-->

(To be filled by the team)

---

## Directory Layout

```
<!-- Replace with your actual structure -->
src/
├── ...
└── ...
```

---

## Module Organization

### Convention: Prebuilt Editor UI Lives Under `mora/src/mora/ui/`

**What**: Keep Mora's default display-protocol UI in the `crate::mora::ui`
module. The public `ui_node::build_ui` entry point is a compatibility shim;
new prebuilt UI components should be added as focused files under
`mora/src/mora/ui/`.

**Why**: Mora uses `display-protocol-jsx` to compose editor chrome. Splitting
prebuilt pieces prevents `ui_node.rs` from becoming a single large renderer and
makes menu bar, modeline, echo area, editor buffer area, and shell UI easier to
extend independently.

**Example**:

```rust
// mora/src/mora/ui/mod.rs
mod menu_bar;
mod modeline;

pub fn build_ui(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let menu = menu_bar::build_menu_bar(editor, width);
    let modeline = modeline::build_modeline(editor, width);
    // compose with display_protocol_jsx::jsx!
}
```

**Current files**:
- `ui/mod.rs`: top-level UI layout and shared JSX helpers.
- `ui/menu_bar.rs`: Emacs-style menu bar.
- `ui/modeline.rs`: Emacs-style modeline.
- `ui/echo_area.rs`: minibuffer and echo area.
- `ui/editor_area.rs`: buffer rows, gutter, cursor, selection, syntax spans.
- `ui/mshell_area.rs`: shell output and shell prompt echo area.

### Contract: Lisp UI Customization Hooks

**Scope / Trigger**: Mora Lisp can customize the display-protocol UI at either
the whole-frame level or at named component slots. Use component slots when Lisp
should redesign chrome while keeping Rust defaults for the editor buffer.

**Signatures**:

```clojure
(mora.ui/register-ui-builder fn)              ; append whole-frame builder
(mora.ui/clear-ui-builders)
(mora.ui/set-ui-component-builder component fn)
(mora.ui/clear-ui-component-builder component)
(mora.ui/clear-ui-component-builders)
(mora.ui/ui-component-builders)               ; => vector of component keywords
```

**Builder Contract**:
- Builders receive `(width height)`.
- Builders return a Lisp UI value convertible by `lisp_value_to_uinode`.
- Returning `nil` from a component builder means "use the Rust default".
- Component names are keywords, symbols, or strings.

**Named Components**:
- `:frame`: replaces the entire default frame after legacy whole-frame builders.
- `:menu-bar`: replaces the top menu bar.
- `:editor-area`: replaces the buffer display area.
- `:shell-area`: replaces mshell output.
- `:modeline`: replaces the modeline.
- `:echo-area`: replaces the editor echo/minibuffer area.
- `:shell-echo-area`: replaces mshell's prompt area, with `:echo-area` as fallback.

**Example**:

```clojure
(require [mora.ui :as ui])

(ui/set-ui-component-builder :modeline
  (fn mora-modeline [width height]
    (text "--:**-  *scratch*  (Mora Lisp)  All"
          {:fg "#ffffff" :bg "#000000" :bold true})))
```

**Tests Required**:
- A component builder overrides a named slot.
- A component builder returning `nil` falls back to the Rust default.
- Lisp UI maps using nested `:props` apply layout fields like width/height/gap.

---

## Naming Conventions

<!-- File and folder naming rules -->

(To be filled by the team)

---

## Examples

<!-- Link to well-organized modules as examples -->

(To be filled by the team)
