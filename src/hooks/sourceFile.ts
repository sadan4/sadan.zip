import { unreachable } from "@/utils/error";
import { extensionForLanguage, Language } from "@/utils/textmate";
import { defaultScriptTarget, getTextChanges, scriptKindForLanguage } from "@/utils/typescript";

import { useEffect, useReducer, useRef } from "react";
import { createSourceFile, type SourceFile, updateSourceFile } from "typescript";

export const enum UpdateType {
    CODE,
    INIT,
    LANGUAGE,
    SCRIPT_TARGET = LANGUAGE,
}

type Update =
  | {
      type: UpdateType.CODE;
      oldCode: string;
  }
  | {
      type: UpdateType.INIT;
  }
  | {
      type: UpdateType.LANGUAGE | UpdateType.SCRIPT_TARGET;
  };

// dumb react strict mode workaround
const operatedOn = new WeakMap<SourceFile, SourceFile>();

interface SourceFileMetadata {
    reparseCount: number;
}

export function useSourceFile(
    code: string,
    language: Language,
    scriptTarget = defaultScriptTarget(language),
): [SourceFile, SourceFileMetadata] {
    const codeRef = useRef(code);
    const languageRef = useRef(language);
    const scriptTargetRef = useRef(scriptTarget);

    // TODO: make useCounter hook
    const [reparseCount, incrementReparseCount] = useReducer(
        (c: number, reset: boolean = false) => +!reset && c + 1,
        0,
    );

    // empty object as source file to make weak map happy
    const [sourceFile, dispatch] = useReducer(reducer, undefined, () => reducer({} as any, { type: UpdateType.INIT }));

    useEffect(() => {
        const oldCode = codeRef.current;

        if (oldCode === code) {
            return;
        }
        codeRef.current = code;
        dispatch({
            type: UpdateType.CODE,
            oldCode,
        });
    }, [code]);

    useEffect(() => {
        if (languageRef.current === language && scriptTargetRef.current === scriptTarget) {
            return;
        }
        languageRef.current = language;
        scriptTargetRef.current = scriptTarget;
        dispatch({
            // OR UpdateType.SCRIPT_TARGET
            // both have the same effect right now
            type: UpdateType.LANGUAGE,
        });
    }, [language, scriptTarget]);

    return [sourceFile, { reparseCount }];

    // eslint-disable-next-line react-hooks/todo
    function reducer(state: SourceFile, action: Update): SourceFile {
        const filename = `file${extensionForLanguage(languageRef.current)}`;
        const newCode = codeRef.current;

        if (import.meta.env.DEV && operatedOn.has(state)) {
            return operatedOn.get(state)!;
        }

        let res: SourceFile | undefined;

        switch (action.type) {
            case UpdateType.CODE: {
                const { oldCode } = action;
                const textChanges = getTextChanges(oldCode, newCode);

                incrementReparseCount();
                res = updateSourceFile(state, newCode, textChanges);
                break;
            }
            case UpdateType.INIT:
            case UpdateType.LANGUAGE:
            case UpdateType.SCRIPT_TARGET: {
                incrementReparseCount(true);
                res = createSourceFile(
                    filename,
                    newCode,
                    scriptTargetRef.current,
                    true,
                    scriptKindForLanguage(languageRef.current),
                );
                break;
            }
            default:
                unreachable();
        }
        if (import.meta.env.DEV) {
            operatedOn.set(state, res);
        }
        return res;
    }
}
