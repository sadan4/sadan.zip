import { z } from "zod";

export const MessageBase = z.object({
    type: z.string(),
});

const WithMessageId = z.object({
    messageId: z.number(),
});

export type MessageBase = z.infer<typeof MessageBase>;

export const TBundleHash = z.string().brand<"bundleHash", "inout">();

export type TBundleHash = z.infer<typeof TBundleHash>;

export const TModuleId = z.string().brand<"moduleId", "inout">();

export type TModuleId = z.infer<typeof TModuleId>;


export const QueryBundlesMessage = MessageBase.safeExtend({
    type: z.literal("queryBundles"),
});

export type QueryBundlesMessage = z.infer<typeof QueryBundlesMessage>;

export const GetBundleMetadataMessage = MessageBase.extend({
    type: z.literal("getBundleMetadata"),
    bundleHash: TBundleHash,
});

export type GetBundleMetadataMessage = z.infer<typeof GetBundleMetadataMessage>;

export const GetBundleDepGraphMessage = MessageBase.extend({
    type: z.literal("getBundleDepGraph"),
    bundleHash: TBundleHash,
});

export type GetBundleDepGraphMessage = z.infer<typeof GetBundleDepGraphMessage>;

export const GetAllBundleFilesMessage = MessageBase.extend({
    type: z.literal("getAllBundleFiles"),
    bundleHash: TBundleHash,
});

export type GetAllBundleFilesMessage = z.infer<typeof GetAllBundleFilesMessage>;

export const GetBundleFileMessage = MessageBase.extend({
    type: z.literal("getBundleFile"),
    bundleHash: TBundleHash,
    moduleNumber: TModuleId,
});

export type GetBundleFileMessage = z.infer<typeof GetBundleFileMessage>;

export const GetBundleArchiveMessage = MessageBase.extend({
    type: z.literal("getBundleArchive"),
    bundleHash: TBundleHash,
});

export type GetBundleArchiveMessage = z.infer<typeof GetBundleArchiveMessage>;

const BaseMessageToServer = z.discriminatedUnion("type", [
    QueryBundlesMessage,
    GetBundleMetadataMessage,
    GetBundleDepGraphMessage,
    GetAllBundleFilesMessage,
    GetBundleFileMessage,
    GetBundleArchiveMessage,
]);

export const MessageToServer = z.intersection(WithMessageId, BaseMessageToServer);

export type BaseMessageToServer = z.infer<typeof BaseMessageToServer>;

export type MessageToServer = z.infer<typeof MessageToServer>;

export const ModuleInfo = z.record(z.string(), z.array(TModuleId));

export type ModuleInfo = z.infer<typeof ModuleInfo>;

/**
 * schema for info.json
 */
export const BundleInfo = z.object({
    buildHash: TBundleHash,
    buildNumber: z.string(),
    firstSeen: z.number(),
    /**
     * The entry point of the module, May be null on bundles parsed before this field was added
     * or if the entry point could not be found
     */
    entryPoint: TModuleId.nullable(),
    /**
     * can't be serialized as it contains symbols, but is cheap to parse, and guaranteed to be valid
     */
    envVarText: z.string(),
});

export const KeyModules = z.object({
    /**
     * [moduleId, exportName][]
     */
    fluxDispatcherClass: z.array(z.tuple([TModuleId, /* exportName */ z.union([z.string(), z.symbol()])])),
});

export type KeyModules = z.infer<typeof KeyModules>;

export const MainDeps = z.record(TModuleId, z.object({
    syncUses: z.array(TModuleId),
    lazyUses: z.array(TModuleId),
}));

export type MainDeps = z.infer<typeof MainDeps>;

export const DepsJson = z.object({
    deps: MainDeps,
    keyModules: KeyModules,
});

export type DepsJson = z.infer<typeof DepsJson>;

export type BundleInfo = z.infer<typeof BundleInfo>;

export const BundlesResponseMessage = MessageBase.extend({
    type: z.literal("queryBundlesResponse"),
    bundles: z.array(BundleInfo),
});

export type BundlesResponseMessage = z.infer<typeof BundlesResponseMessage>;

export const AllBundleFilesResponseMessage = MessageBase.extend({
    type: z.literal("getAllBundleFilesResponse"),
    bundleHash: TBundleHash,
    files: z.record(TModuleId, z.string()),
});

export type AllBundleFilesResponseMessage = z.infer<typeof AllBundleFilesResponseMessage>;

export const BundleMetadataResponseMessage = MessageBase.extend({
    type: z.literal("getBundleMetadataResponse"),
    bundleHash: TBundleHash,
    metadata: BundleInfo,
    moduleInfo: ModuleInfo,
});

export type BundleMetadataResponseMessage = z.infer<typeof BundleMetadataResponseMessage>;

export const BundleDepGraphResponseMessage = MessageBase.extend({
    type: z.literal("getBundleDepGraphResponse"),
    bundleHash: TBundleHash,
    depGraph: DepsJson,
});

export type BundleDepGraphResponseMessage = z.infer<typeof BundleDepGraphResponseMessage>;

export const BundleFileResponseMessage = MessageBase.extend({
    type: z.literal("getBundleFileResponse"),
    bundleHash: TBundleHash,
    moduleNumber: TModuleId,
    fileText: z.string(),
});

export type BundleFileResponseMessage = z.infer<typeof BundleFileResponseMessage>;

export const BundleArchiveResponseMessage = MessageBase.extend({
    type: z.literal("getBundleArchiveResponse"),
    bundleHash: TBundleHash,
    b64: z.string(),
});

export type BundleArchiveResponseMessage = z.infer<typeof BundleArchiveResponseMessage>;


export const ErrorMessage = MessageBase.extend({
    type: z.literal("error"),
    message: z.string(),
});

export type ErrorMessage = z.infer<typeof ErrorMessage>;

const BaseMessageToClient = z.discriminatedUnion("type", [
    BundlesResponseMessage,
    AllBundleFilesResponseMessage,
    BundleMetadataResponseMessage,
    BundleDepGraphResponseMessage,
    BundleFileResponseMessage,
    BundleArchiveResponseMessage,
    ErrorMessage,
]);

export type BaseMessageToClient = z.infer<typeof BaseMessageToClient>;

export const MessageToClient = z.intersection(WithMessageId, BaseMessageToClient);

export type MessageToClient = z.infer<typeof MessageToClient>;

