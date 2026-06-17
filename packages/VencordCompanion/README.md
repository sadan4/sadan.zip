# **REQUIRES A CUSTOM USERPLUGIN, SEE LINK BELOW**

[https://github.com/sadan4/vc-userDevTools/tree/main](https://github.com/sadan4/vc-userDevTools/blob/main)

> **Architecture note (in-flight migration).** The extension has been split
> into two pieces: a thin VSCode client (`src/extension.ts`) and a Rust
> language server (`companion_lsp`) that lives in the parent workspace at
> `../../crates/companion_lsp/`. The server owns all parsing/analysis and the
> Discord WebSocket bridge; the VSCode client only handles editor-specific
> UI (QuickPick, tree view, diff viewer, custom Patch Helper editor).
>
> The server speaks standard LSP over stdio, plus a small set of custom
> `vencord/*` JSON-RPC methods documented in
> `crates/companion_lsp/src/vencord_ext.rs`. Other editors (Neovim, Helix,
> Zed) can implement those custom methods in their own shims to get the
> full feature set.
>
> Build the server with `cargo build -p companion_lsp` in the workspace
> root. For local development point the extension at the binary via the
> `vencord-user-companion.lspPath` setting or the `COMPANION_LSP_BIN`
> environment variable.

# VencordCompanion

[Download on the VSCode marketplace](https://marketplace.visualstudio.com/items?itemName=sadan.vencord-user-companion)

[Download on the Open VSX marketplace](https://open-vsx.org/extension/sadan/vencord-user-companion)

![image](https://user-images.githubusercontent.com/45497981/224365555-60e968a1-d2d0-4aee-b29b-e5714273682c.png)

![image](https://user-images.githubusercontent.com/45497981/224377149-b1569eac-9411-4f55-849a-950ba5b06f37.png)

## Features

-   Testing Patches
-   Diffing Patches
-   Extracting Webpack Modules
    -   From Patches
    -   From Finds
-   Disable/Enable Plugin buttons above the definePlugin export
-   Automatically run the reporter and have a gui with with the results
-   Webpack LSP that lets you jump around extracted webpack files
-   See where exports from a webpack module are used
-   Cache discords modules locally

## Images/Videos of the Features

### Webpack LSP

https://github.com/user-attachments/assets/7d4ab157-0751-4a59-8e0e-1a3607a3247d

### Reporter Gui

https://github.com/user-attachments/assets/71c45fda-5161-43b0-8b2d-6e5fae8267d2

### Testing Patches

https://github.com/user-attachments/assets/99a9157e-89bb-45c7-b780-ffac30cdf4d0

### Diffing Patches
#### Only works for patches that are currently applied and have not errored
#### Shows every patch to that webpack module, not just yours

https://github.com/user-attachments/assets/958f4b61-4390-47fa-9dd3-6fc888dc844d

### Extracting Webpack Modules
#### Use the toggle in the plugin setting to default to the extracted module or the unpatched module if the module is patched

https://github.com/user-attachments/assets/bbe308c8-af9a-4141-b387-9dcf175cfd25

### Disable/Enable Plugins
#### There is a plugin setting to set auto-reload after a plugin is toggled

https://github.com/user-attachments/assets/56de9c1d-fb6d-4665-aff0-6429f80d1f15

### Module Cache
#### To enable the side bar, use the settings in vscode

https://github.com/user-attachments/assets/950230e6-64e5-4bbf-86bf-384ac0b3857d

### Jumping to References

https://github.com/user-attachments/assets/e82291c4-bad3-479c-bfd0-8810d07faab9

