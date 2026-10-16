export enum TextmateTheme {
    TOKYO_NIGHT,
    ROSE_PINE,
    ROSE_PINE_DAWN,
    ROSE_PINE_MOON,
    NORD,
    CATPPUCCIN_MOCHA,
    // This *MUST* be after catppuccin mocha so the enum numbers stay sequential
    CATPPUCCIN = CATPPUCCIN_MOCHA,
    CATPPUCCIN_FRAPPE,
    CATPPUCCIN_MACCHIATO,
    CATPPUCCIN_LATTE,
    DRACULA,
    GRUVBOX,
    OXOCARBON,
    VSCODE_DARK,
    VSCODE_LIGHT,
}

export const DEFAULT_THEME = TextmateTheme.TOKYO_NIGHT;

export const themeDisplayNames = Object.freeze({
    [TextmateTheme.TOKYO_NIGHT]: "Tokyo Night",
    [TextmateTheme.ROSE_PINE]: "Rose Pine",
    [TextmateTheme.ROSE_PINE_DAWN]: "Rose Pine Dawn",
    [TextmateTheme.ROSE_PINE_MOON]: "Rose Pine Moon",
    [TextmateTheme.NORD]: "Nord",
    [TextmateTheme.CATPPUCCIN_MOCHA]: "Catppuccin Mocha",
    [TextmateTheme.CATPPUCCIN_FRAPPE]: "Catppuccin Frappe",
    [TextmateTheme.CATPPUCCIN_MACCHIATO]: "Catppuccin Macchiato",
    [TextmateTheme.CATPPUCCIN_LATTE]: "Catppuccin Latte",
    [TextmateTheme.GRUVBOX]: "Gruvbox",
    [TextmateTheme.OXOCARBON]: "Oxocarbon",
    [TextmateTheme.DRACULA]: "Dracula",
    [TextmateTheme.VSCODE_DARK]: "VSCode Dark",
    [TextmateTheme.VSCODE_LIGHT]: "VSCode Light",
} satisfies Record<TextmateTheme, string>);
