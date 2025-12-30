export interface QueryBundlesMessage {
    type: "queryBundles";
}

export interface GetBundleMetadataMessage {
    type: "getBundleMetadata";
    bundleHash: string;
}

export interface GetBundleDepGraphMessage {
    type: "getBundleDepGraph";
    bundleHash: string;
}

export interface GetAllBundleFilesMessage {
    type: "getAllBundleFiles";
    bundleHash: string;
}

export interface GetBundleFileMessage {
    type: "getBundleFile";
    bundleHash: string;
    moduleNumber: string;
}

export type MessageToServer =
  | QueryBundlesMessage
  | GetAllBundleFilesMessage
  | GetBundleMetadataMessage
  | GetBundleDepGraphMessage
  | GetBundleFileMessage;

export interface ModuleInfo {
    [jsFilePath: string]: readonly string[];
}

export interface BundleInfo {
    buildHash: string;
    buildNumber: string;
    firstSeen: number;
    modules: ModuleInfo;
    /**
     * can't be serialized as it contains symbols, but is cheap to parse, and guaranteed to be valid
     */
    envVarText: string;
}

export interface BundlesResponseMessage {
    type: "queryBundlesResponse";
    bundles: BundleInfo[];
}

export interface AllBundleFilesResponseMessage {
    type: "getAllBundleFilesResponse";
    bundleHash: string;
    files: {
        [moduleNumber: string]: string;
    };
}

export interface BundleMetadataResponseMessage {
    type: "getBundleMetadataResponse";
    bundleHash: string;
    metadata: BundleInfo;
}

export interface BundleDepGraphResponseMessage {
    type: "getBundleDepGraphResponse";
    bundleHash: string;
    depGraph: DepsJson;
}

export interface BundleFileResponseMessage {
    type: "getBundleFileResponse";
    bundleHash: string;
    moduleNumber: string;
    fileText: string;
}

export interface ErrorMessage {
    type: "error";
    sourceType: string;
    message: string;
}

export type MessageToClient =
  | BundlesResponseMessage
  | AllBundleFilesResponseMessage
  | BundleFileResponseMessage
  | BundleMetadataResponseMessage
  | BundleDepGraphResponseMessage
  | ErrorMessage;

export interface KeyModules {
    fluxDispatcherClass: [moduleId: string, exportName: string | symbol][];
}

export interface MainDeps {
    [key: string]: {
        syncUses: string[];
        lazyUses: string[];
    };
}

export interface DepsJson {
    deps: MainDeps;
    keyModules: KeyModules;
}
