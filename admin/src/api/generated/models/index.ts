/* tslint:disable */
/* eslint-disable */
/**
 *
 * @export
 * @interface AgentEnrollmentResponse
 */
export interface AgentEnrollmentResponse {
    /**
     *
     * @type {AgentResponse}
     * @memberof AgentEnrollmentResponse
     */
    agent: AgentResponse;
    /**
     *
     * @type {string}
     * @memberof AgentEnrollmentResponse
     */
    enrollmentExpiresAt: string;
    /**
     *
     * @type {string}
     * @memberof AgentEnrollmentResponse
     */
    enrollmentToken: string;
    /**
     *
     * @type {string}
     * @memberof AgentEnrollmentResponse
     */
    installCommand: string;
}
/**
 *
 * @export
 * @interface AgentInstallCommandResponse
 */
export interface AgentInstallCommandResponse {
    /**
     *
     * @type {string}
     * @memberof AgentInstallCommandResponse
     */
    agentId: string;
    /**
     *
     * @type {string}
     * @memberof AgentInstallCommandResponse
     */
    enrollmentExpiresAt: string;
    /**
     *
     * @type {string}
     * @memberof AgentInstallCommandResponse
     */
    enrollmentToken: string;
    /**
     *
     * @type {string}
     * @memberof AgentInstallCommandResponse
     */
    installCommand: string;
}
/**
 *
 * @export
 * @interface AgentListResponse
 */
export interface AgentListResponse {
    /**
     *
     * @type {Array<AgentResponse>}
     * @memberof AgentListResponse
     */
    items: Array<AgentResponse>;
    /**
     *
     * @type {string}
     * @memberof AgentListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface AgentResponse
 */
export interface AgentResponse {
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    agentVersion?: string | null;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    architecture?: string | null;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    createdAt: string;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    hostname?: string | null;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    lastSeenAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    name: string;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    nodeId: string;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    registeredAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    revokedAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof AgentResponse
     */
    status: string;
}
/**
 *
 * @export
 * @interface ApplicationGrantListResponse
 */
export interface ApplicationGrantListResponse {
    /**
     *
     * @type {Array<ApplicationGrantResponse>}
     * @memberof ApplicationGrantListResponse
     */
    items: Array<ApplicationGrantResponse>;
    /**
     *
     * @type {string}
     * @memberof ApplicationGrantListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface ApplicationGrantResponse
 */
export interface ApplicationGrantResponse {
    /**
     *
     * @type {string}
     * @memberof ApplicationGrantResponse
     */
    applicationId: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationGrantResponse
     */
    grantedAt: string;
}
/**
 *
 * @export
 * @interface ApplicationListResponse
 */
export interface ApplicationListResponse {
    /**
     *
     * @type {Array<ApplicationResponse>}
     * @memberof ApplicationListResponse
     */
    items: Array<ApplicationResponse>;
    /**
     *
     * @type {string}
     * @memberof ApplicationListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface ApplicationResponse
 */
export interface ApplicationResponse {
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    createdAt: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    description: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    name: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    slug: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    status: string;
    /**
     *
     * @type {string}
     * @memberof ApplicationResponse
     */
    updatedAt: string;
    /**
     *
     * @type {number}
     * @memberof ApplicationResponse
     */
    version: number;
}
/**
 *
 * @export
 * @interface ApplicationStatusRequest
 */
export interface ApplicationStatusRequest {
    /**
     *
     * @type {string}
     * @memberof ApplicationStatusRequest
     */
    status: string;
    /**
     *
     * @type {number}
     * @memberof ApplicationStatusRequest
     */
    version: number;
}
/**
 *
 * @export
 * @interface AuditLogListResponse
 */
export interface AuditLogListResponse {
    /**
     *
     * @type {Array<AuditLogResponse>}
     * @memberof AuditLogListResponse
     */
    items: Array<AuditLogResponse>;
    /**
     *
     * @type {string}
     * @memberof AuditLogListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface AuditLogResponse
 */
export interface AuditLogResponse {
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    action: string;
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    actorId?: string | null;
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    createdAt: string;
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    requestId: string;
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    resourceId: string;
    /**
     *
     * @type {string}
     * @memberof AuditLogResponse
     */
    resourceType: string;
    /**
     *
     * @type {any}
     * @memberof AuditLogResponse
     */
    summary: any | null;
}
/**
 *
 * @export
 * @interface ConfirmRequest
 */
export interface ConfirmRequest {
    /**
     *
     * @type {any}
     * @memberof ConfirmRequest
     */
    parameters: any | null;
    /**
     *
     * @type {string}
     * @memberof ConfirmRequest
     */
    snapshotHash: string;
}
/**
 *
 * @export
 * @interface CreateAgentRequest
 */
export interface CreateAgentRequest {
    /**
     *
     * @type {string}
     * @memberof CreateAgentRequest
     */
    name: string;
    /**
     *
     * @type {string}
     * @memberof CreateAgentRequest
     */
    nodeId?: string | null;
}
/**
 *
 * @export
 * @interface CreateUserRequest
 */
export interface CreateUserRequest {
    /**
     *
     * @type {string}
     * @memberof CreateUserRequest
     */
    displayName?: string | null;
    /**
     *
     * @type {string}
     * @memberof CreateUserRequest
     */
    email?: string | null;
    /**
     *
     * @type {string}
     * @memberof CreateUserRequest
     */
    password: string;
    /**
     *
     * @type {string}
     * @memberof CreateUserRequest
     */
    username: string;
}
/**
 *
 * @export
 * @interface CsrfTokenResponse
 */
export interface CsrfTokenResponse {
    /**
     *
     * @type {string}
     * @memberof CsrfTokenResponse
     */
    csrfToken: string;
}
/**
 *
 * @export
 * @interface DeploymentListResponse
 */
export interface DeploymentListResponse {
    /**
     *
     * @type {Array<DeploymentResponse>}
     * @memberof DeploymentListResponse
     */
    items: Array<DeploymentResponse>;
    /**
     *
     * @type {string}
     * @memberof DeploymentListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface DeploymentLogResponse
 */
export interface DeploymentLogResponse {
    /**
     *
     * @type {string}
     * @memberof DeploymentLogResponse
     */
    content: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentLogResponse
     */
    createdAt: string;
    /**
     *
     * @type {number}
     * @memberof DeploymentLogResponse
     */
    sequence: number;
    /**
     *
     * @type {string}
     * @memberof DeploymentLogResponse
     */
    stream: string;
    /**
     *
     * @type {boolean}
     * @memberof DeploymentLogResponse
     */
    truncated: boolean;
}
/**
 *
 * @export
 * @interface DeploymentPreviewResponse
 */
export interface DeploymentPreviewResponse {
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    applicationId: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    applicationName: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    environment: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    nodeId: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    nodeName: string;
    /**
     *
     * @type {any}
     * @memberof DeploymentPreviewResponse
     */
    parameters: any | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    scriptPath: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    snapshotHash: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentPreviewResponse
     */
    targetId: string;
}
/**
 *
 * @export
 * @interface DeploymentResponse
 */
export interface DeploymentResponse {
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    cancelRequestedAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    createdAt: string;
    /**
     *
     * @type {number}
     * @memberof DeploymentResponse
     */
    exitCode?: number | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    finishedAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    phase: string;
    /**
     *
     * @type {boolean}
     * @memberof DeploymentResponse
     */
    protocolComplete: boolean;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    queuedAt: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    requestedBy: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    resultSummary?: string | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    retryOfId?: string | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    snapshotHash: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    startedAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    status: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    targetId: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentResponse
     */
    updatedAt: string;
    /**
     *
     * @type {number}
     * @memberof DeploymentResponse
     */
    version: number;
}
/**
 *
 * @export
 * @interface DeploymentTargetListResponse
 */
export interface DeploymentTargetListResponse {
    /**
     *
     * @type {Array<DeploymentTargetResponse>}
     * @memberof DeploymentTargetListResponse
     */
    items: Array<DeploymentTargetResponse>;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface DeploymentTargetResponse
 */
export interface DeploymentTargetResponse {
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    applicationId: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    createdAt: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    environment: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    nodeId: string;
    /**
     *
     * @type {any}
     * @memberof DeploymentTargetResponse
     */
    parameterSchema: any | null;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    scriptPath: string;
    /**
     *
     * @type {Array<SecretFileReference>}
     * @memberof DeploymentTargetResponse
     */
    secretFileReferences: Array<SecretFileReference>;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    snapshotHash: string;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    status: string;
    /**
     *
     * @type {number}
     * @memberof DeploymentTargetResponse
     */
    timeoutSeconds: number;
    /**
     *
     * @type {string}
     * @memberof DeploymentTargetResponse
     */
    updatedAt: string;
    /**
     *
     * @type {any}
     * @memberof DeploymentTargetResponse
     */
    verificationConfig: any | null;
    /**
     *
     * @type {number}
     * @memberof DeploymentTargetResponse
     */
    version: number;
}
/**
 *
 * @export
 * @interface EnrollRequest
 */
export interface EnrollRequest {
    /**
     *
     * @type {string}
     * @memberof EnrollRequest
     */
    agentId: string;
    /**
     *
     * @type {string}
     * @memberof EnrollRequest
     */
    agentVersion: string;
    /**
     *
     * @type {string}
     * @memberof EnrollRequest
     */
    architecture: string;
    /**
     *
     * @type {string}
     * @memberof EnrollRequest
     */
    enrollmentToken: string;
    /**
     *
     * @type {string}
     * @memberof EnrollRequest
     */
    hostname: string;
    /**
     *
     * @type {string}
     * @memberof EnrollRequest
     */
    os: string;
    /**
     *
     * @type {number}
     * @memberof EnrollRequest
     */
    protocolVersion: number;
}
/**
 *
 * @export
 * @interface ErrorResponse
 */
export interface ErrorResponse {
    /**
     *
     * @type {string}
     * @memberof ErrorResponse
     */
    code: string;
    /**
     *
     * @type {any}
     * @memberof ErrorResponse
     */
    details?: any | null;
    /**
     *
     * @type {string}
     * @memberof ErrorResponse
     */
    message: string;
    /**
     *
     * @type {string}
     * @memberof ErrorResponse
     */
    requestId: string;
}
/**
 *
 * @export
 * @interface LoginRequest
 */
export interface LoginRequest {
    /**
     *
     * @type {string}
     * @memberof LoginRequest
     */
    password: string;
    /**
     *
     * @type {string}
     * @memberof LoginRequest
     */
    username: string;
}
/**
 *
 * @export
 * @interface NodeCheckResponse
 */
export interface NodeCheckResponse {
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    architecture?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    createdAt: string;
    /**
     *
     * @type {number}
     * @memberof NodeCheckResponse
     */
    diskAvailableBytes?: number | null;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    failureCode?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    failureMessage?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    finishedAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    osName?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeCheckResponse
     */
    status: string;
}
/**
 *
 * @export
 * @interface NodeListResponse
 */
export interface NodeListResponse {
    /**
     *
     * @type {Array<NodeResponse>}
     * @memberof NodeListResponse
     */
    items: Array<NodeResponse>;
    /**
     *
     * @type {string}
     * @memberof NodeListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface NodeResponse
 */
export interface NodeResponse {
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    checkedAt?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    createdAt: string;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    host?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    name: string;
    /**
     *
     * @type {number}
     * @memberof NodeResponse
     */
    port?: number | null;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    secretsRoot?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    sshCredentialId?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    status: string;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    trustedHostFingerprint?: string | null;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    updatedAt: string;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    username?: string | null;
    /**
     *
     * @type {number}
     * @memberof NodeResponse
     */
    version: number;
    /**
     *
     * @type {string}
     * @memberof NodeResponse
     */
    workRoot?: string | null;
}
/**
 *
 * @export
 * @interface PreviewRequest
 */
export interface PreviewRequest {
    /**
     *
     * @type {any}
     * @memberof PreviewRequest
     */
    parameters: any | null;
}
/**
 *
 * @export
 * @interface RefreshRequest
 */
export interface RefreshRequest {
    /**
     *
     * @type {string}
     * @memberof RefreshRequest
     */
    refreshToken: string;
    /**
     *
     * @type {string}
     * @memberof RefreshRequest
     */
    rotationId: string;
}
/**
 *
 * @export
 * @interface RefreshTokenPairResponse
 */
export interface RefreshTokenPairResponse {
    /**
     *
     * @type {string}
     * @memberof RefreshTokenPairResponse
     */
    accessExpiresAt: string;
    /**
     *
     * @type {string}
     * @memberof RefreshTokenPairResponse
     */
    accessToken: string;
    /**
     *
     * @type {string}
     * @memberof RefreshTokenPairResponse
     */
    agentId: string;
    /**
     *
     * @type {string}
     * @memberof RefreshTokenPairResponse
     */
    refreshExpiresAt: string;
    /**
     *
     * @type {string}
     * @memberof RefreshTokenPairResponse
     */
    refreshToken: string;
    /**
     *
     * @type {string}
     * @memberof RefreshTokenPairResponse
     */
    rotationId: string;
}
/**
 *
 * @export
 * @interface ResetPasswordRequest
 */
export interface ResetPasswordRequest {
    /**
     *
     * @type {string}
     * @memberof ResetPasswordRequest
     */
    password: string;
    /**
     *
     * @type {number}
     * @memberof ResetPasswordRequest
     */
    version: number;
}
/**
 *
 * @export
 * @interface RuntimeSettings
 */
export interface RuntimeSettings {
    /**
     *
     * @type {number}
     * @memberof RuntimeSettings
     */
    logRetentionDays: number;
    /**
     *
     * @type {number}
     * @memberof RuntimeSettings
     */
    maxConcurrentDeployments: number;
    /**
     *
     * @type {number}
     * @memberof RuntimeSettings
     */
    maxLogBytes: number;
    /**
     *
     * @type {number}
     * @memberof RuntimeSettings
     */
    version: number;
}
/**
 *
 * @export
 * @interface SaveApplicationRequest
 */
export interface SaveApplicationRequest {
    /**
     *
     * @type {string}
     * @memberof SaveApplicationRequest
     */
    description?: string;
    /**
     *
     * @type {string}
     * @memberof SaveApplicationRequest
     */
    name: string;
    /**
     *
     * @type {string}
     * @memberof SaveApplicationRequest
     */
    slug: string;
    /**
     *
     * @type {number}
     * @memberof SaveApplicationRequest
     */
    version?: number | null;
}
/**
 *
 * @export
 * @interface SaveTargetRequest
 */
export interface SaveTargetRequest {
    /**
     *
     * @type {string}
     * @memberof SaveTargetRequest
     */
    environment: string;
    /**
     *
     * @type {string}
     * @memberof SaveTargetRequest
     */
    nodeId: string;
    /**
     *
     * @type {any}
     * @memberof SaveTargetRequest
     */
    parameterSchema: any | null;
    /**
     *
     * @type {string}
     * @memberof SaveTargetRequest
     */
    scriptPath: string;
    /**
     *
     * @type {Array<SecretFileReference>}
     * @memberof SaveTargetRequest
     */
    secretFileReferences?: Array<SecretFileReference>;
    /**
     *
     * @type {number}
     * @memberof SaveTargetRequest
     */
    timeoutSeconds: number;
    /**
     *
     * @type {any}
     * @memberof SaveTargetRequest
     */
    verificationConfig: any | null;
    /**
     *
     * @type {number}
     * @memberof SaveTargetRequest
     */
    version?: number | null;
}
/**
 *
 * @export
 * @interface SecretFileReference
 */
export interface SecretFileReference {
    /**
     *
     * @type {string}
     * @memberof SecretFileReference
     */
    environmentKey: string;
    /**
     *
     * @type {string}
     * @memberof SecretFileReference
     */
    filePath: string;
}
/**
 *
 * @export
 * @interface SessionResponse
 */
export interface SessionResponse {
    /**
     *
     * @type {string}
     * @memberof SessionResponse
     */
    csrfToken: string;
    /**
     *
     * @type {UserIdentity}
     * @memberof SessionResponse
     */
    user: UserIdentity;
}
/**
 *
 * @export
 * @interface SetupRequest
 */
export interface SetupRequest {
    /**
     *
     * @type {string}
     * @memberof SetupRequest
     */
    displayName?: string | null;
    /**
     *
     * @type {string}
     * @memberof SetupRequest
     */
    email?: string | null;
    /**
     *
     * @type {string}
     * @memberof SetupRequest
     */
    password: string;
    /**
     *
     * @type {string}
     * @memberof SetupRequest
     */
    username: string;
}
/**
 *
 * @export
 * @interface SetupStatusResponse
 */
export interface SetupStatusResponse {
    /**
     *
     * @type {boolean}
     * @memberof SetupStatusResponse
     */
    setupRequired: boolean;
}
/**
 *
 * @export
 * @interface SshCredentialListResponse
 */
export interface SshCredentialListResponse {
    /**
     *
     * @type {Array<SshCredentialResponse>}
     * @memberof SshCredentialListResponse
     */
    items: Array<SshCredentialResponse>;
    /**
     *
     * @type {string}
     * @memberof SshCredentialListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface SshCredentialResponse
 */
export interface SshCredentialResponse {
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    algorithm: string;
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    createdAt: string;
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    fingerprint: string;
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    name: string;
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    publicKey: string;
    /**
     *
     * @type {string}
     * @memberof SshCredentialResponse
     */
    updatedAt: string;
    /**
     *
     * @type {number}
     * @memberof SshCredentialResponse
     */
    version: number;
}
/**
 *
 * @export
 * @interface StatusResponse
 */
export interface StatusResponse {
    /**
     *
     * @type {string}
     * @memberof StatusResponse
     */
    status: string;
}
/**
 *
 * @export
 * @interface TargetStatusRequest
 */
export interface TargetStatusRequest {
    /**
     *
     * @type {string}
     * @memberof TargetStatusRequest
     */
    status: string;
    /**
     *
     * @type {number}
     * @memberof TargetStatusRequest
     */
    version: number;
}
/**
 *
 * @export
 * @interface TokenPairResponse
 */
export interface TokenPairResponse {
    /**
     *
     * @type {string}
     * @memberof TokenPairResponse
     */
    accessExpiresAt: string;
    /**
     *
     * @type {string}
     * @memberof TokenPairResponse
     */
    accessToken: string;
    /**
     *
     * @type {string}
     * @memberof TokenPairResponse
     */
    agentId: string;
    /**
     *
     * @type {string}
     * @memberof TokenPairResponse
     */
    refreshExpiresAt: string;
    /**
     *
     * @type {string}
     * @memberof TokenPairResponse
     */
    refreshToken: string;
}
/**
 *
 * @export
 * @interface UpdateProfileRequest
 */
export interface UpdateProfileRequest {
    /**
     *
     * @type {string}
     * @memberof UpdateProfileRequest
     */
    displayName: string;
}
/**
 *
 * @export
 * @interface UpdateStatusRequest
 */
export interface UpdateStatusRequest {
    /**
     *
     * @type {string}
     * @memberof UpdateStatusRequest
     */
    status: string;
    /**
     *
     * @type {number}
     * @memberof UpdateStatusRequest
     */
    version: number;
}
/**
 *
 * @export
 * @interface UpdateUserPreferencesRequest
 */
export interface UpdateUserPreferencesRequest {
    /**
     *
     * @type {boolean}
     * @memberof UpdateUserPreferencesRequest
     */
    followLogs: boolean;
    /**
     *
     * @type {boolean}
     * @memberof UpdateUserPreferencesRequest
     */
    notifyDeploymentCompleted: boolean;
    /**
     *
     * @type {boolean}
     * @memberof UpdateUserPreferencesRequest
     */
    notifyDeploymentFailed: boolean;
    /**
     *
     * @type {boolean}
     * @memberof UpdateUserPreferencesRequest
     */
    notifyNodeUnhealthy: boolean;
    /**
     *
     * @type {string}
     * @memberof UpdateUserPreferencesRequest
     */
    timeFormat: string;
    /**
     *
     * @type {number}
     * @memberof UpdateUserPreferencesRequest
     */
    version: number;
}
/**
 *
 * @export
 * @interface UserIdentity
 */
export interface UserIdentity {
    /**
     *
     * @type {string}
     * @memberof UserIdentity
     */
    displayName: string;
    /**
     *
     * @type {string}
     * @memberof UserIdentity
     */
    email?: string | null;
    /**
     *
     * @type {string}
     * @memberof UserIdentity
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof UserIdentity
     */
    identity: string;
    /**
     *
     * @type {string}
     * @memberof UserIdentity
     */
    username: string;
}
/**
 *
 * @export
 * @interface UserListResponse
 */
export interface UserListResponse {
    /**
     *
     * @type {Array<UserResponse>}
     * @memberof UserListResponse
     */
    items: Array<UserResponse>;
    /**
     *
     * @type {string}
     * @memberof UserListResponse
     */
    nextCursor?: string | null;
}
/**
 *
 * @export
 * @interface UserPreferencesResponse
 */
export interface UserPreferencesResponse {
    /**
     *
     * @type {boolean}
     * @memberof UserPreferencesResponse
     */
    followLogs: boolean;
    /**
     *
     * @type {boolean}
     * @memberof UserPreferencesResponse
     */
    notifyDeploymentCompleted: boolean;
    /**
     *
     * @type {boolean}
     * @memberof UserPreferencesResponse
     */
    notifyDeploymentFailed: boolean;
    /**
     *
     * @type {boolean}
     * @memberof UserPreferencesResponse
     */
    notifyNodeUnhealthy: boolean;
    /**
     *
     * @type {string}
     * @memberof UserPreferencesResponse
     */
    timeFormat: string;
    /**
     *
     * @type {number}
     * @memberof UserPreferencesResponse
     */
    version: number;
}
/**
 *
 * @export
 * @interface UserResponse
 */
export interface UserResponse {
    /**
     *
     * @type {string}
     * @memberof UserResponse
     */
    displayName: string;
    /**
     *
     * @type {string}
     * @memberof UserResponse
     */
    email?: string | null;
    /**
     *
     * @type {string}
     * @memberof UserResponse
     */
    id: string;
    /**
     *
     * @type {string}
     * @memberof UserResponse
     */
    identity: string;
    /**
     *
     * @type {string}
     * @memberof UserResponse
     */
    status: string;
    /**
     *
     * @type {string}
     * @memberof UserResponse
     */
    username: string;
    /**
     *
     * @type {number}
     * @memberof UserResponse
     */
    version: number;
}
