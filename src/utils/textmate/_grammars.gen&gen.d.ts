/* eslint-disable */
import * as shiki from 'shiki';
import { Language } from './language.ts';

type LazyLang = shiki.LanguageRegistration;

declare const languagesWithGrammars: Set<Language>;
declare function json(): Promise<LazyLang>;
declare function js(): Promise<LazyLang>;
declare function ts(): Promise<LazyLang>;
declare function tsx(): Promise<LazyLang>;
declare function jsx(): Promise<LazyLang>;
declare function html(): Promise<LazyLang>;
declare function css(): Promise<LazyLang>;

export { css, html, js, json, jsx, languagesWithGrammars, ts, tsx };
export type { LazyLang };
