// @ts-nocheck
import "monaco-editor/esm/vs/platform/theme/common/colorRegistry";
// @ts-expect-error monaco internal api
import { Color } from "monaco-editor/esm/vs/base/common/color";
// @ts-expect-error internal
import * as bt from "monaco-editor/esm/vs/editor/standalone/common/themes";
// @ts-expect-error monaco internal api
import { Registry as _Registry } from "monaco-editor/esm/vs/platform/registry/common/platform";
import { ColorScheme } from "monaco-editor/esm/vs/platform/theme/common/theme";

const VS_LIGHT_THEME_NAME = "vs";
// const VS_DARK_THEME_NAME = "vs-dark";
const HC_BLACK_THEME_NAME = "hc-black";
const HC_LIGHT_THEME_NAME = "hc-light";

interface IRegistry {
    /**
     * Returns the extension functions and properties defined by the specified key or null.
     * @param id an extension identifier
     */
    as<T>(id: string): T;
}

declare class Color {
    __color_brand: unknown;
}
declare namespace Color {
    export namespace Format {
        export namespace CSS {
            export function formatHexA(c: Color): string;
        }
    }
}

interface ColorContribution {
    readonly id: string;

    // ... other
}

interface IColorRegistry {
    /**
     * Get all color contributions
     */
    getColors(): ColorContribution[];
    /**
     * Gets the default color of the given id
     */
    resolveDefaultColor(id: string, theme: IColorTheme): Color | undefined;
}

const Registry: IRegistry = _Registry;
const colorRegistry = Registry.as<IColorRegistry>("base.contributions.colors");

interface IColorTheme {
    __color_theme_brand: unknown;
}

interface IStandaloneTheme extends IColorTheme {
    __standalone_theme_brand: unknown;
}

class StandaloneTheme implements IStandaloneTheme {
    declare __color_theme_brand: unknown;
    declare __standalone_theme_brand: unknown;

    constructor(name, standaloneThemeData) {
        this.semanticHighlighting = false;
        this.themeData = standaloneThemeData;

        const { base } = standaloneThemeData;

        if (name.length > 0) {
            this.id = name;
            this.themeName = name;
        } else {
            this.id = base;
            this.themeName = base;
        }
        this.colors = null;
        this.defaultColors = Object.create(null);
        this._tokenTheme = null;
    }

    get base() {
        return this.themeData.base;
    }

    notifyBaseUpdated() {
        if (this.themeData.inherit) {
            this.colors = null;
            this._tokenTheme = null;
        }
    }

    getColors() {
        if (!this.colors) {
            const colors = new Map();

            for (const id in this.themeData.colors) {
                colors.set(id, Color.fromHex(this.themeData.colors[id]));
            }
            if (this.themeData.inherit) {
                const baseData = getBuiltinRules(this.themeData.base);

                for (const id in baseData.colors) {
                    if (!colors.has(id)) {
                        colors.set(id, Color.fromHex(baseData.colors[id]));
                    }
                }
            }
            this.colors = colors;
        }
        return this.colors;
    }

    getColor(colorId, useDefault) {
        const color = this.getColors().get(colorId);

        if (color) {
            return color;
        }
        if (useDefault !== false) {
            return this.getDefault(colorId);
        }
        return undefined;
    }

    getDefault(colorId) {
        let color = this.defaultColors[colorId];

        if (color) {
            return color;
        }
        color = colorRegistry.resolveDefaultColor(colorId, this);
        this.defaultColors[colorId] = color;
        return color;
    }

    defines(colorId) {
        return this.getColors().has(colorId);
    }

    get type() {
        switch (this.base) {
            case VS_LIGHT_THEME_NAME:
                return ColorScheme.LIGHT;
            case HC_BLACK_THEME_NAME:
                return ColorScheme.HIGH_CONTRAST_DARK;
            case HC_LIGHT_THEME_NAME:
                return ColorScheme.HIGH_CONTRAST_LIGHT;
            default:
                return ColorScheme.DARK;
        }
    }

    get tokenTheme() {
        if (!this._tokenTheme) {
            let rules = [];
            let encodedTokensColors = [];

            if (this.themeData.inherit) {
                const baseData = getBuiltinRules(this.themeData.base);

                rules = baseData.rules;
                if (baseData.encodedTokensColors) {
                    encodedTokensColors = baseData.encodedTokensColors;
                }
            }

            // Pick up default colors from `editor.foreground` and `editor.background` if available
            const editorForeground = this.themeData.colors["editor.foreground"];
            const editorBackground = this.themeData.colors["editor.background"];

            if (editorForeground || editorBackground) {
                const rule = { token: "" };

                if (editorForeground) {
                    rule.foreground = editorForeground;
                }
                if (editorBackground) {
                    rule.background = editorBackground;
                }
                rules.push(rule);
            }
            rules = rules.concat(this.themeData.rules);
            if (this.themeData.encodedTokensColors) {
                encodedTokensColors = this.themeData.encodedTokensColors;
            }
            this._tokenTheme = TokenTheme.createFromRawTokenTheme(
                rules,
                encodedTokensColors,
            );
        }
        return this._tokenTheme;
    }

    getTokenStyleMetadata(type, modifiers, _modelLanguage) {
        // use theme rules match
        const style = this.tokenTheme._match([type].concat(modifiers).join("."));
        const { metadata } = style;
        const foreground = TokenMetadata.getForeground(metadata);
        const fontStyle = TokenMetadata.getFontStyle(metadata);

        return {
            foreground,
            italic: Boolean(fontStyle & 1 /* FontStyle.Italic */),
            bold: Boolean(fontStyle & 2 /* FontStyle.Bold */),
            underline: Boolean(fontStyle & 4 /* FontStyle.Underline */),
            strikethrough: Boolean(fontStyle & 8 /* FontStyle.Strikethrough */),
        };
    }

    get tokenColorMap() {
        return [];
    }
}

const vs = new StandaloneTheme("vs", bt.vs);
const vs_dark = new StandaloneTheme("vs-dark", bt.vs_dark);

export function getBaseTheme(type?: "dark" | "light"): [key: string, color: string][] {
    const colors = colorRegistry.getColors();
    let theme: IStandaloneTheme;

    switch (type) {
        case "dark": {
            theme = vs_dark;
            break;
        }
        case "light": {
            theme = vs;
            break;
        }
        case undefined: {
            error(`could not get base theme for base ${tmTheme.type}`);
        }
    }

    const ret: [key: string, color?: string][] = [];

    for (const color of colors) {
        const cv = colorRegistry.resolveDefaultColor(color.id, theme);

        if (cv == null) {
            console.warn(`no default for: ${JSON.stringify(color.id)}`);
            ret.push([color.id, void 0]);
            continue;
        }

        const hexStr = Color.Format.CSS.formatHexA(cv);

        ret.push([color.id, hexStr]);
    }
    return ret;
}
