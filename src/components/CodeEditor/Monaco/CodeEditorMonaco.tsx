import { useControlledState } from "@/hooks/controlledState";
import { useLock } from "@/hooks/lock";
import { useRecent } from "@/hooks/recent";
import { useResizeObserver } from "@/hooks/resizeObserver";
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

import { Suspense, use, useCallback, useEffect, useImperativeHandle, useLayoutEffect, useMemo, useRef, useState } from "react";

export interface MonacoCodeEditorProps extends CodeEditorProps<MonacoCodeEditor.Handle> {
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
    const editorRef = useRef<Monaco.editor.IStandaloneCodeEditor>(null);
    const themeString = useMonacoTheme(theme);
    const lock = useLock();
    const decorationsRef = useRef<Monaco.editor.IEditorDecorationsCollection | null>(null);
    const onDidChangeCursorPositionRef = useRecent(onDidChangeCursorPosition);

    const [code, setCode] = useControlledState({
        initialValue: initialCode,
        managedValue: userCode,
        handleChange: onChange,
        debugName: "MonacoCodeEditor.code",
    });

    function handleEditorDidMount() {
        assert(editorRef.current);
        editorRef.current.onDidChangeCursorPosition((e) => {
            onDidChangeCursorPositionRef.current(e);
        });
        editorRef.current.onDidChangeModelContent(lock.bindIf(() => {
            const text = editorRef.current?.getModel()?.getValue() ?? "";

            setCode(text);
        }));
    }

    useImperativeHandle(_ref, () => ({
        // eslint-disable-next-line react-hooks/todo
        get editor() {
            return editorRef.current!;
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
        let timeout: NodeJS.Timeout;

        if (ref == null) {
            return;
        }

        const mergedOptions: Monaco.editor.IStandaloneEditorConstructionOptions = {
            ...options,
        };

        // eslint-disable-next-line logical-assignment-operators -- make react compiler happy
        if (!mergedOptions.model) {
            mergedOptions.model = (uri && monaco.editor.getModel(uri)) || monaco.editor.createModel(
                code,
                getMonacoLanguageString(language),
                uri ?? uriForLanguage(language),
            );
        }
        // eslint-disable-next-line logical-assignment-operators -- make react compiler happy
        if (mergedOptions.extraEditorClassName == null) {
            mergedOptions.extraEditorClassName = className;
        }
        editorRef.current = monaco.editor.create(ref, {
            ...mergedOptions,
            theme: themeString,
        });
        // @ts-expect-error
        if (window.requestIdleCallback) {
            requestIdleCallback(() => {
                setupThemes(editorRef.current!);
            }, {
                timeout: 2000,
            });
        } else {
            timeout = setTimeout(() => {
                setupThemes(editorRef.current!);
            }, 2000);
        }
        handleEditorDidMount();

        return () => clearTimeout(timeout);
        // TODO: look into what deps are needed here
        // eslint-disable-next-line @eslint-react/exhaustive-deps
    }, [ref]);

    useEffect(() => {
        if (!editorRef.current) {
            return;
        }
        setupThemes(editorRef.current);
    }, [setupThemes]);

    useEffect(() => {
        editorRef.current?.updateOptions({
            extraEditorClassName: className ?? "",
        });
    }, [className]);

    useEffect(() => {
        if (!editorRef.current) {
            return;
        }

        const { model: _, ...newOptions } = options;

        editorRef.current.updateOptions(newOptions);
    }, [options]);

    const style = useMemo(() => ({
        width,
        height,
    }), [height, width]);

    useLayoutEffect(() => {
        editorRef.current?.layout();
    }, [style]);

    useResizeObserver(ref, () => {
        editorRef.current?.layout();
    });

    useEffect(() => () => {
        editorRef.current?.dispose();
    }, []);

    useEffect(() => {
        const model = editorRef.current?.getModel();

        if (!model) {
            return;
        }

        updateModelLanguage(model, language);
    }, [language]);

    useEffect(() => {
        const model = editorRef.current?.getModel();

        if (!model) {
            return;
        }

        // TODO: should we use option.model as a marker for fully user controlled instead
        // don't do anything if the user is managing the model
        if (model.uri.toString() === uri?.toString()) {
            return;
            // the user provided a uri and it doesn't match the current model
        } else if (uri) {
            const newModel = monaco.editor.getModel(uri);

            if (!newModel) {
                console.warn("user changed uri but there is no model associated with it");
                return;
            }

            editorRef.current?.setModel(newModel);
            return;
        }

        const modelText = model.getValue();

        if (modelText === code) {
            return;
        }

        const e = editorRef.current!;
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
    }, [code, uri, lock]);

    useEffect(() => {
        if (!editorRef.current) {
            return;
        }
        if (!highlights?.length) {
            decorationsRef.current?.clear();
            return;
        }

        // eslint-disable-next-line logical-assignment-operators -- make react compiler happy
        if (decorationsRef.current == null) {
            decorationsRef.current = editorRef.current.createDecorationsCollection();
        }

        const newDecorations = highlights.map((range) => ({
            range,
            options: {
                className: styles.highlight,
            },
        }) satisfies Monaco.editor.IModelDeltaDecoration);

        decorationsRef.current.set(newDecorations);
    }, [highlights]);

    return (
        <div
            style={style}
            ref={setRef}
            className="h-full w-full contain-inline-size"
            data-code-editor="monaco"
        />
    );
}

export namespace MonacoCodeEditor {
    export interface Handle {
        get editor(): Monaco.editor.IStandaloneCodeEditor;
    }
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
