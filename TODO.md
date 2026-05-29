/auto-loop implement menu-bar-mode for mora, it's like emacs's menu-bar-mode, you should follow display-protocol to implement it

# Mora coldnew-emacs replacement gaps

Mora cannot replace `ref/coldnew-emacs` yet. The syntax and command model are improving, but the missing work is mostly editor capability surface.

## Recommended priority

1. Loadable Mora Lisp namespaces from disk.
2. Interactive command args and a real M-x picker.
3. Multiple buffers plus `find-file` and `switch-buffer`.
4. Mode hooks plus Lisp-defined commands/keymaps.
5. Project and ripgrep picker.
6. LSP basics.
7. Git basics.
8. Native AI commands.

## Configuration runtime

- Add `load-file` and disk-backed `require` for Mora Lisp namespaces.
- Add a load-path equivalent.
- Expose user config/cache directories to Lisp.
- Add a Mora-native config variable system.
- Improve command docs and command discovery UI.
- Add interactive argument prompts for commands.

## M-x and minibuffer

- Add a real command palette UI.
- Add fuzzy completion.
- Show command docs while selecting commands.
- Add prompt readers for interactive command args.
- Add command history.
- Support argument readers for string, number, file, directory, buffer, command, and choice.

## Buffers, files, projects

- Make multiple buffers first-class.
- Add buffer list and switch-buffer.
- Add recent files.
- Add project root detection.
- Add find-file with completion.
- Add save-some-buffers.
- Add auto-revert.
- Add backup/autosave policy.
- Add sudo/root edit equivalent.
- Add rename/delete current file commands.

## Windows, tabs, workspaces

- Add tab-bar/workspace equivalent.
- Add named workspaces.
- Add persistent layouts.
- Add robust split resize/focus commands.
- Improve buffer-per-window semantics.
- Expose window operations to Lisp.

## Editing commands

- Expose cleanup-buffer to Lisp.
- Add indent-whole-buffer.
- Add untabify.
- Add eval-and-replace.
- Expose copy-and-comment.
- Expose dos2unix/unix2dos as Lisp commands.
- Expose narrowing APIs.
- Expose rectangle APIs.
- Expose mark/region APIs.
- Add better kill-ring APIs.

## Major and minor modes

- Define major modes from Lisp.
- Define minor modes from Lisp.
- Add mode hooks.
- Add file pattern mode associations.
- Add syntax table/comment style APIs.
- Add mode-local keymaps.
- Add buffer-local variables.

## Completion, search, UI

- Add fuzzy command/file/buffer picker.
- Add ripgrep UI.
- Add consult-line style line search.
- Add symbol search.
- Add completion-at-point.
- Add completion popup.
- Add orderless/flexible matching.
- Add preview UI.

## Language tooling

- Add LSP client integration.
- Add diagnostics UI.
- Add goto definition/references.
- Add xref equivalent.
- Add formatting commands.
- Add tree-sitter indentation/folding.
- Add per-language docs/devdocs equivalent.

## Git, shell, terminal

- Add git status UI.
- Add diff hunks.
- Add stage/unstage/commit.
- Add blame.
- Add terminal buffer.
- Add shell buffer using `mishell`.
- Add shell command output buffers.

## AI and agent integration

- Add inline suggestions.
- Add chat buffer.
- Add apply-patch command from assistant output.
- Add explain-region command.
- Add generate-commit-message command.
- Add project-aware agent commands.
- Add model/provider config.
