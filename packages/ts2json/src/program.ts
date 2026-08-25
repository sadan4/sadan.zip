import { CompilerHost, CompilerOptions, createCompilerHost, createProgram, createSourceFile, ModuleKind, ModuleResolutionKind, Program, ScriptKind, ScriptTarget, SourceFile } from "typescript";

export const DEFAULT_COMPILER_OPTIONS: CompilerOptions = {
    target: ScriptTarget.ESNext,
    module: ModuleKind.ESNext,
    moduleResolution: ModuleResolutionKind.Bundler,
    strict: true,
    skipLibCheck: true,
};

/**
 * must end with a supported extension,
 * or it will be silently ignored
 */
const FILE_NAME = "/\x01FILE.ts";


export interface VirtualProgram { 
    program: Program;
    host: CompilerHost;
    rootFile: SourceFile;
}

export function createVirtualProgram(
    tsCode: string,
    options: CompilerOptions = DEFAULT_COMPILER_OPTIONS,
    scriptKind = ScriptKind.TS
): VirtualProgram {
    const host = createCompilerHost(options, true);
    const sourceFile = createSourceFile(FILE_NAME, tsCode, options.target ?? ScriptTarget.ESNext, true, scriptKind);
    const { getSourceFile, fileExists, readFile } = host;
    host.getSourceFile = function (fileName) { 
        if (fileName === FILE_NAME) {
            return sourceFile;
        } else { 
            return getSourceFile.apply(this ?? host, arguments as any);
        }
    }
    host.fileExists = function (fileName) { 
        if (fileName === FILE_NAME) {
            return true;
        } else { 
            return fileExists.apply(this ?? host, arguments as any);
        }
    }
    host.readFile = function (fileName) { 
        if (fileName === FILE_NAME) {
            return tsCode;
        } else { 
            return readFile.apply(this ?? host, arguments as any);
        }
    }
    host.writeFile = function (fileName) { 
        console.warn(`writeFile called for ${fileName}, but this is a virtual program. Ignoring.`);
    }
    const program = createProgram({
        options: options,
        rootNames: [FILE_NAME],
        host,
    });
    // sanity check: make sure it was added to the program
    // the program silently discards any file that doesnt end with a supported extension
    const rootFile = program.getSourceFile(FILE_NAME);
    if (!rootFile) {
        throw new Error(`virtual root file was not added to the program`);
    }
    return {
        program,
        host,
        rootFile
    };
}
