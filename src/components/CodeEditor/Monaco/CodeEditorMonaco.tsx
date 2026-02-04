import { useControlledState } from "@/hooks/controlledState";
import { useLock } from "@/hooks/lock";
import { useRecent } from "@/hooks/recent";
import { useRect } from "@/hooks/rect";
import { EMPTY_NULL_OBJECT, NOOP } from "@/utils/constants";
import { assert } from "@/utils/error";
import { once } from "@/utils/functional";
import { getMonacoLanguageString, isReadOnly, makeTMLanguageMap, type Monaco, monaco, updateModelLanguage, uriForLanguage } from "@/utils/monaco";
import { loadOnigasmPromise } from "@/utils/oniguruma";
import { hasGrammar, Language } from "@/utils/textmate";
import { ClientOnly } from "@tanstack/react-router";

import { registry, wireTmGrammars } from "./grammars";
import styles from "./styles.module.scss";
import { DEFAULT_MONACO_THEME, type MonacoTheme, useMonacoTheme } from "./themes";
import { type CodeEditorProps } from "../base";

import { Suspense, use, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";

export interface MonacoCodeEditorHandle {
    get editor(): Monaco.editor.IStandaloneCodeEditor;
}

export interface MonacoCodeEditorProps extends CodeEditorProps<MonacoCodeEditorHandle> {
    theme?: MonacoTheme;
    options?: Monaco.editor.IStandaloneEditorConstructionOptions;
    uri?: Monaco.Uri;
    highlights?: Monaco.IRange[];
    onDidChangeCursorPosition?: (e: Monaco.editor.ICursorPositionChangedEvent) => void;
}

const monacoSetup = once(() => {
    monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
        jsx: monaco.languages.typescript.JsxEmit.Preserve,
    });
    monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: true,
        noSyntaxValidation: false,
    });
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
    highlights,
    onDidChangeCursorPosition = NOOP,
    ref: _ref,
}: MonacoCodeEditorProps) {
    !import.meta.env.SSR && monacoSetup();
    use(loadOnigasmPromise());

    const [ref, setRef] = useState<HTMLDivElement | null>(null);
    const rect = useRect(ref);
    const editor = useRef<Monaco.editor.IStandaloneCodeEditor>(null);
    const themeString = useMonacoTheme(theme);
    const lock = useLock();
    const decorations = useRef<Monaco.editor.IEditorDecorationsCollection | null>(null);
    const onDidChangeCursorPositionRef = useRecent(onDidChangeCursorPosition);

    const [code, setCode] = useControlledState({
        initialValue: initialCode,
        managedValue: userCode,
        handleChange: onChange,
        debugName: "MonacoCodeEditor.code",
    });

    function handleEditorDidMount() {
        assert(editor.current);
        editor.current.onDidChangeCursorPosition((e) => {
            onDidChangeCursorPositionRef.current(e);
        });
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

    const setupThemes = useCallback(function updateLanguages(editor: Monaco.editor.ICodeEditor) {
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

        const mergedOptions: Monaco.editor.IStandaloneEditorConstructionOptions = {
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

    useEffect(() => {
        if (!editor.current) {
            return;
        }
        if (!highlights?.length) {
            decorations.current?.clear();
            return;
        }
        decorations.current ??= editor.current.createDecorationsCollection();

        const newDecorations = highlights.map((range) => ({
            range,
            options: {
                className: styles.highlight,
            },
        }) satisfies Monaco.editor.IModelDeltaDecoration);

        decorations.current.set(newDecorations);
    }, [highlights]);

    return (
        <div
            style={style}
            ref={setRef}
            className="h-full w-full"
            data-code-editor="monaco"
        />
    );
}

export function MonacoCodeEditor(props: MonacoCodeEditorProps) {
    return (
        <Suspense fallback={null}>
            <ClientOnly>
                <MonacoCodeEditorInner {...props} />
            </ClientOnly>
        </Suspense>
    );
}
