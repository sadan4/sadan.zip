/* eslint-disable */
import * as shiki from 'shiki';
import { Language } from './language.ts';

type LazyLang = shiki.LanguageRegistration[];

declare const languagesWithGrammars: Set<Language>;
declare function json(): Promise<LazyLang>;
declare function javascript(): Promise<LazyLang>;
declare function typescript(): Promise<LazyLang>;
declare function typescriptreact(): Promise<LazyLang>;
declare function javascriptreact(): Promise<LazyLang>;
declare function html(): Promise<LazyLang>;

export { html, javascript, javascriptreact, json, languagesWithGrammars, typescript, typescriptreact };
export type { LazyLang };
