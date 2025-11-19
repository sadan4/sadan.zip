# vscode-textmate

This is a vendor of the `vscode-textmate` npm package. It is needed because `vscode-textmate` is published on npm as a single-line bundled and minified javascript file. It exposes some internal classes for use and makes a minor tweak to `Theme` to allow an existing `ColorMap` to be used instead of having to proxy an array. All other changes are just eslint/formatting. The source code can be found at `https://github.com/microsoft/vscode-textmate/tree/76ab07aecfbd7e959ee4b55de3976f7a3ee95f38`
