/* eslint-disable */
import * as monaco_editor from 'monaco-editor';
import { TextmateTheme } from '@/utils/textmate/theme';

type MonacoThemeData = monaco_editor.editor.IStandaloneThemeData;

declare function TOKYO_NIGHT(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function ROSE_PINE(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function ROSE_PINE_DAWN(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function ROSE_PINE_MOON(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function NORD(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPPUCCIN(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPPUCCIN_FRAPPE(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPPUCCIN_MACCHIATO(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPPUCCIN_LATTE(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function DRACULA(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function GRUVBOX(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function OXOCARBON(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare const loaderMap: Record<TextmateTheme, () => Promise<MonacoThemeData>>;

export { CATPPUCCIN, CATPPUCCIN_FRAPPE, CATPPUCCIN_LATTE, CATPPUCCIN_MACCHIATO, DRACULA, GRUVBOX, NORD, OXOCARBON, ROSE_PINE, ROSE_PINE_DAWN, ROSE_PINE_MOON, TOKYO_NIGHT, loaderMap };
export type { MonacoThemeData };
