# Lessons

- Display widget examples must render through a real backend. Do not replace TUI component work with stdout preview strings or backend-specific placeholder widgets.
- `display-protocol-widgets` owns backend-neutral widget state; `display-tui` owns terminal rendering and examples.
