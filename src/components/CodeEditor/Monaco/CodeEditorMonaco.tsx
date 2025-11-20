import { useControlledState } from "@/hooks/controlledState";
import { useLock } from "@/hooks/lock";
import { useRect } from "@/hooks/rect";
import { EMPTY_NULL_OBJECT } from "@/utils/constants";
import { assert, error } from "@/utils/error";
import { once } from "@/utils/functional";
import { getMonacoLanguageString, isReadOnly, makeTMLanguageMap, updateModelLanguage, uriForLanguage } from "@/utils/monaco";
import { loadOnigasmPromise } from "@/utils/oniguruma";
import { hasGrammar, Language } from "@/utils/textmate";

import { registry, wireTmGrammars } from "./grammars";
import { DEFAULT_MONACO_THEME, type MonacoTheme, useMonacoTheme } from "./themes";
import { type CodeEditorProps } from "../base";

import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import { Suspense, use, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";

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

function MonacoCodeEditorInner({
    initialCode = "",
    code: userCode,
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
    use(loadOnigasmPromise());

    const [ref, setRef] = useState<HTMLDivElement | null>(null);
    const rect = useRect(ref);
    const editor = useRef<monaco.editor.IStandaloneCodeEditor>(null);
    const themeString = useMonacoTheme(theme);
    const lock = useLock();

    const [code, setCode] = useControlledState({
        initialValue: initialCode,
        managedValue: userCode,
        handleChange: onChange,
        debugName: "MonacoCodeEditor.code",
    });

    function handleEditorDidMount() {
        assert(editor.current);
        editor.current.onDidChangeModelContent(lock.bindIf(() => {
            const text = editor.current?.getModel()?.getValue() ?? "";

            setCode(text);
        }));
    }

    useImperativeHandle(_ref, () => ({
        get editor() {
            return editor.current!;
        },
    }));

    const setupThemes = useCallback(function updateLanguages(editor: monaco.editor.ICodeEditor) {
        if (!hasGrammar(language)) {
            return;
        }

        const langDepsMap = makeTMLanguageMap(language);

        wireTmGrammars(registry(), langDepsMap, editor, themeString)
            .then(() => monaco.editor.setTheme(themeString));
    }, [language, themeString]);


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

                updateModelLanguage(model, language);
            } else {
                model = monaco.editor.createModel(
                    code,
                    getMonacoLanguageString(language),
                    uri ?? uriForLanguage(language),
                );
            }
            mergedOptions.model = model;
        }
        mergedOptions.extraEditorClassName ??= className;
        editor.current = monaco.editor.create(ref, {
            ...mergedOptions,
            theme: themeString,
        });
        requestIdleCallback(() => {
            setupThemes(editor.current!);
        }, {
            timeout: 2000,
        });
        handleEditorDidMount();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [ref]);

    useEffect(() => {
        if (!editor.current) {
            return;
        }
        setupThemes(editor.current);
    }, [setupThemes]);

    useEffect(() => {
        editor.current?.updateOptions({
            extraEditorClassName: className ?? "",
        });
    }, [className]);

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

    useEffect(() => {
        const model = editor.current?.getModel();

        if (!model) {
            return;
        }

        updateModelLanguage(model, language);
    }, [language]);

    useEffect(() => {
        const model = editor.current?.getModel();

        if (!model) {
            return;
        }

        const modelText = model.getValue();

        if (modelText === code) {
            return;
        }

        const e = editor.current!;
        const readOnly = isReadOnly(e);

        lock.lockWhile(() => {
            if (readOnly) {
                model.setValue(code);
            } else {
                e.executeEdits("", [
                    {
                        range: model.getFullModelRange(),
                        text: code,
                        forceMoveMarkers: true,
                    },
                ]);
                e.pushUndoStop();
            }
        });
    }, [code, lock]);

    return (
        <div
            ref={setRef}
            style={style}
            data-code-editor="monaco"
        />
    );
}

export function MonacoCodeEditor(props: MonacoCodeEditorProps) {
    return (
        <Suspense fallback={null}>
            <MonacoCodeEditorInner {...props} />
        </Suspense>
    );
}
