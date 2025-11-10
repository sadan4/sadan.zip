import { useControlledState } from "@/hooks/controlledState";
import { EMPTY_NULL_OBJECT } from "@/utils/constants";
import { assert } from "@/utils/error";

import { DEFAULT_MONACO_THEME, type MonacoTheme, useThemeString } from "./themes";
import { type CodeEditorProps, Language } from "../base";

import * as monaco from "monaco-editor";
import { useEffect, useImperativeHandle, useMemo, useRef } from "react";

export interface MonacoCodeEditorHandle {
    get editor(): monaco.editor.IStandaloneCodeEditor;
}

export interface MonacoCodeEditorProps extends CodeEditorProps<MonacoCodeEditorHandle> {
    theme?: MonacoTheme;
    options?: monaco.editor.IStandaloneEditorConstructionOptions;
    uri?: monaco.Uri;
}

export function MonacoCodeEditor({
    initialCode = "",
    language = Language.UNKNOWN,
    onChange,
    theme = DEFAULT_MONACO_THEME,
    width = "100%",
    height = "100%",
    options = EMPTY_NULL_OBJECT,
    className,
    uri,
    ref: _ref,
}: MonacoCodeEditorProps) {
    const ref = useRef<HTMLDivElement>(null);
    const editor = useRef<monaco.editor.IStandaloneCodeEditor>(null);
    const themeString = useThemeString(theme);

    const [code, setCode] = useControlledState({
        initialValue: initialCode,
        managedValue: undefined,
        handleChange: onChange,
        debugName: "MonacoCodeEditor.code",
    });

    function handleEditorDidMount() {
        assert(editor.current);
        editor.current.onDidChangeModelContent(() => {
            const text = editor.current?.getModel()?.getValue() ?? "";

            setCode(text);
        });
    }

    useImperativeHandle(_ref, () => ({
        get editor() {
            return editor.current!;
        },
    }));

    useEffect(() => {
        if (ref.current == null) {
            return;
        }

        const mergedOptions: monaco.editor.IStandaloneEditorConstructionOptions = {
            ...options,
        };

        if (!mergedOptions.model) {
            let model = uri && monaco.editor.getModel(uri);

            if (model) {
                model.setValue(code);

                const langStr = getLanguageString(language);

                if (langStr) {
                    monaco.editor.setModelLanguage(model, langStr);
                }
            } else {
                model = monaco.editor.createModel(code, getLanguageString(language), uri);
            }
            mergedOptions.model = model;
        }
        mergedOptions.extraEditorClassName ??= className;
        editor.current = monaco.editor.create(ref.current, {
            ...mergedOptions,
            theme: themeString,
        });
        handleEditorDidMount();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // useEffect(() => {
    //     if (!editor.current) {
    //         return;
    //     }
    //     if (code === editor.current.getValue()) {
    //         return;
    //     }

    //     const model = editor.current.getModel();

    //     // lock = true;
    //     editor.current.pushUndoStop();
    //     model?.pushEditOperations(
    //         [],
    //         [
    //             {
    //                 range: model.getFullModelRange(),
    //                 text: code,
    //             },
    //         ],
    //         () => null,
    //     );
    //     editor.current.pushUndoStop();
    //     // lock = false;
    // }, [code]);

    useEffect(() => {
        editor.current?.updateOptions({
            extraEditorClassName: className ?? "",
        });
    }, [className]);

    useEffect(() => {
        monaco.editor.setTheme(themeString);
    }, [themeString]);

    useEffect(() => {
        if (!editor.current) {
            return;
        }

        const { model: _, ...newOptions } = options;

        editor.current.updateOptions(newOptions);
    }, [options]);

    const style = useMemo(() => ({
        width,
        height,
    }), [height, width]);

    useEffect(() => {
        editor.current?.layout();
    }, [style]);

    useEffect(() => () => {
        editor.current?.dispose();
    }, []);

    return (
        <div
            ref={ref}
            style={style}
            data-code-editor="monaco"
        />
    );
}

// generate with
// ```js
// const j = monaco.languages.getLanguages().map(({ id }) => id);
// const tmp = j.map(lang => `[Language.${lang.toUpperCase().replaceAll(/[.-]/g, "_")}]: ${JSON.stringify(lang)},`);
// tmp.push(`[Language.UNKNOWN]: undefined,`);
// const result = tmp.join("\n");
// ```
const LanguageMap: Record<Language, string | undefined> = {
    [Language.PLAINTEXT]: "plaintext",
    [Language.JSON]: "json",
    [Language.ABAP]: "abap",
    [Language.APEX]: "apex",
    [Language.AZCLI]: "azcli",
    [Language.BAT]: "bat",
    [Language.BICEP]: "bicep",
    [Language.CAMELIGO]: "cameligo",
    [Language.CLOJURE]: "clojure",
    [Language.COFFEESCRIPT]: "coffeescript",
    [Language.C]: "c",
    [Language.CPP]: "cpp",
    [Language.CSHARP]: "csharp",
    [Language.CSP]: "csp",
    [Language.CSS]: "css",
    [Language.CYPHER]: "cypher",
    [Language.DART]: "dart",
    [Language.DOCKERFILE]: "dockerfile",
    [Language.ECL]: "ecl",
    [Language.ELIXIR]: "elixir",
    [Language.FLOW9]: "flow9",
    [Language.FSHARP]: "fsharp",
    [Language.FREEMARKER2]: "freemarker2",
    [Language.FREEMARKER2_TAG_ANGLE_INTERPOLATION_DOLLAR]: "freemarker2.tag-angle.interpolation-dollar",
    [Language.FREEMARKER2_TAG_BRACKET_INTERPOLATION_DOLLAR]: "freemarker2.tag-bracket.interpolation-dollar",
    [Language.FREEMARKER2_TAG_ANGLE_INTERPOLATION_BRACKET]: "freemarker2.tag-angle.interpolation-bracket",
    [Language.FREEMARKER2_TAG_BRACKET_INTERPOLATION_BRACKET]: "freemarker2.tag-bracket.interpolation-bracket",
    [Language.FREEMARKER2_TAG_AUTO_INTERPOLATION_DOLLAR]: "freemarker2.tag-auto.interpolation-dollar",
    [Language.FREEMARKER2_TAG_AUTO_INTERPOLATION_BRACKET]: "freemarker2.tag-auto.interpolation-bracket",
    [Language.GO]: "go",
    [Language.GRAPHQL]: "graphql",
    [Language.HANDLEBARS]: "handlebars",
    [Language.HCL]: "hcl",
    [Language.HTML]: "html",
    [Language.INI]: "ini",
    [Language.JAVA]: "java",
    [Language.JAVASCRIPT]: "javascript",
    [Language.JULIA]: "julia",
    [Language.KOTLIN]: "kotlin",
    [Language.LESS]: "less",
    [Language.LEXON]: "lexon",
    [Language.LUA]: "lua",
    [Language.LIQUID]: "liquid",
    [Language.M3]: "m3",
    [Language.MARKDOWN]: "markdown",
    [Language.MDX]: "mdx",
    [Language.MIPS]: "mips",
    [Language.MSDAX]: "msdax",
    [Language.MYSQL]: "mysql",
    [Language.OBJECTIVE_C]: "objective-c",
    [Language.PASCAL]: "pascal",
    [Language.PASCALIGO]: "pascaligo",
    [Language.PERL]: "perl",
    [Language.PGSQL]: "pgsql",
    [Language.PHP]: "php",
    [Language.PLA]: "pla",
    [Language.POSTIATS]: "postiats",
    [Language.POWERQUERY]: "powerquery",
    [Language.POWERSHELL]: "powershell",
    [Language.PROTO]: "proto",
    [Language.PUG]: "pug",
    [Language.PYTHON]: "python",
    [Language.QSHARP]: "qsharp",
    [Language.R]: "r",
    [Language.RAZOR]: "razor",
    [Language.REDIS]: "redis",
    [Language.REDSHIFT]: "redshift",
    [Language.RESTRUCTUREDTEXT]: "restructuredtext",
    [Language.RUBY]: "ruby",
    [Language.RUST]: "rust",
    [Language.SB]: "sb",
    [Language.SCALA]: "scala",
    [Language.SCHEME]: "scheme",
    [Language.SCSS]: "scss",
    [Language.SHELL]: "shell",
    [Language.SOL]: "sol",
    [Language.AES]: "aes",
    [Language.SPARQL]: "sparql",
    [Language.SQL]: "sql",
    [Language.ST]: "st",
    [Language.SWIFT]: "swift",
    [Language.SYSTEMVERILOG]: "systemverilog",
    [Language.VERILOG]: "verilog",
    [Language.TCL]: "tcl",
    [Language.TWIG]: "twig",
    [Language.TYPESCRIPT]: "typescript",
    [Language.TYPESPEC]: "typespec",
    [Language.VB]: "vb",
    [Language.WGSL]: "wgsl",
    [Language.XML]: "xml",
    [Language.YAML]: "yaml",
    [Language.UNKNOWN]: undefined,
};

function getLanguageString(language: Language): string | undefined {
    return LanguageMap[language];
}
