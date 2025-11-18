/* eslint-disable */
import * as monaco_editor from 'monaco-editor';
import { TextmateTheme } from '@/utils/textmate/theme';

type MonacoThemeData = monaco_editor.editor.IStandaloneThemeData;

declare function TOKYO_NIGHT(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function ROSE_PINE(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function ROSE_PINE_DAWN(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function ROSE_PINE_MOON(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function NORD(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPUCCIN(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPUCCIN_FRAPPE(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPUCCIN_MACCHIATO(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function CATPUCCIN_LATTE(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare function DRACULA(): Promise<monaco_editor.editor.IStandaloneThemeData>;
declare const loaderMap: Record<TextmateTheme, () => Promise<MonacoThemeData>>;

export { CATPUCCIN, CATPUCCIN_FRAPPE, CATPUCCIN_LATTE, CATPUCCIN_MACCHIATO, DRACULA, NORD, ROSE_PINE, ROSE_PINE_DAWN, ROSE_PINE_MOON, TOKYO_NIGHT, loaderMap };
export type { MonacoThemeData };
