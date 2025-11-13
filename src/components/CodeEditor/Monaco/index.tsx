import { useControlledState } from "@/hooks/controlledState";
import { useRect } from "@/hooks/rect";
import { EMPTY_NULL_OBJECT } from "@/utils/constants";
import { assert, error } from "@/utils/error";
import { once } from "@/utils/functional";
import { Language } from "@/utils/textmate";

import { registry } from "./grammars";
import { DEFAULT_MONACO_THEME, type MonacoTheme, useThemeString } from "./themes";
import { type CodeEditorProps } from "../base";

import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import { wireTmGrammars } from "monaco-editor-textmate";
import { useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";

export interface MonacoCodeEditorHandle {
    get editor(): monaco.editor.IStandaloneCodeEditor;
}

export interface MonacoCodeEditorProps extends CodeEditorProps<MonacoCodeEditorHandle> {
    theme?: MonacoTheme;
    options?: monaco.editor.IStandaloneEditorConstructionOptions;
    uri?: monaco.Uri;
}

const monacoSetup = once(() => {
    monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
        jsx: monaco.languages.typescript.JsxEmit.Preserve,
    });
    monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: false,
        noSyntaxValidation: false,
    });
    window.MonacoEnvironment = {
        getWorker(name, label) {
            if (label === "json") {
                return new jsonWorker({ name });
            }
            if (label === "css" || label === "scss" || label === "less") {
                return new cssWorker({ name });
            }
            if (label === "html" || label === "handlebars" || label === "razor") {
                return new htmlWorker({ name });
            }
            if (label === "typescript" || label === "javascript") {
                return new tsWorker({ name });
            }
            if (label === "editorWorkerService") {
                return new editorWorker({ name });
            }
            error(`unknown label ${label}`);
        },
    };
});

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
    monacoSetup();

    const [ref, setRef] = useState<HTMLDivElement | null>(null);
    const rect = useRect(ref);
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
        if (ref == null) {
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
        editor.current = monaco.editor.create(ref, {
            ...mergedOptions,
            theme: themeString,
        });
        updateLanguages(editor.current, language);
        handleEditorDidMount();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [ref]);

    function updateLanguages(editor: monaco.editor.ICodeEditor, lang: Language) {
        wireTmGrammars(monaco, registry(), new Map([[lang, lang]]), editor);
    }

    useEffect(() => {
        if (!editor.current) {
            return;
        }
        updateLanguages(editor.current, language);
    }, [language]);

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
    }, [style, rect]);

    useEffect(() => () => {
        editor.current?.dispose();
    }, []);

    return (
        <div
            ref={setRef}
            style={style}
            data-code-editor="monaco"
        />
    );
}

function getLanguageString(language: Language): string | undefined {
    return language || undefined;
}
