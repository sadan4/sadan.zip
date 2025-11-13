import { type Lazy } from "@/utils/lazy";
import * as shiki from "shiki";
export type LazyLang = Lazy<shiki.LanguageRegistration>;
export declare const json: LazyLang;
export declare const javascript: LazyLang;
export declare const typescript: LazyLang;
export declare const typescriptreact: LazyLang;
export declare const javascriptreact: LazyLang;
export declare const html: LazyLang;
