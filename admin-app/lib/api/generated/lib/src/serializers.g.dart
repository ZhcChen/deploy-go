// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'serializers.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

Serializers _$serializers =
    (Serializers().toBuilder()
          ..add(AgentEnrollmentResponse.serializer)
          ..add(AgentInstallCommandResponse.serializer)
          ..add(AgentListResponse.serializer)
          ..add(AgentReleaseListResponse.serializer)
          ..add(AgentReleaseResponse.serializer)
          ..add(AgentResponse.serializer)
          ..add(ApplicationDeploymentPreviewResponse.serializer)
          ..add(ApplicationEnvFileListResponse.serializer)
          ..add(ApplicationEnvFileResponse.serializer)
          ..add(ApplicationEnvPlaintextResponse.serializer)
          ..add(ApplicationGrantListResponse.serializer)
          ..add(ApplicationGrantResponse.serializer)
          ..add(ApplicationListResponse.serializer)
          ..add(ApplicationResponse.serializer)
          ..add(ApplicationSourceResponse.serializer)
          ..add(ApplicationStatusRequest.serializer)
          ..add(AuditLogListResponse.serializer)
          ..add(AuditLogResponse.serializer)
          ..add(ConfirmRequest.serializer)
          ..add(CreateAgentRequest.serializer)
          ..add(CreateGitCredentialRequest.serializer)
          ..add(CreateUserRequest.serializer)
          ..add(CsrfTokenResponse.serializer)
          ..add(DeleteApplicationEnvRequest.serializer)
          ..add(DeploymentListResponse.serializer)
          ..add(DeploymentLogResponse.serializer)
          ..add(DeploymentPreviewResponse.serializer)
          ..add(DeploymentResponse.serializer)
          ..add(DeploymentStageTaskSummary.serializer)
          ..add(DeploymentTargetListResponse.serializer)
          ..add(DeploymentTargetPreviewResponse.serializer)
          ..add(DeploymentTargetResponse.serializer)
          ..add(DeploymentTargetRunResponse.serializer)
          ..add(EnrollRequest.serializer)
          ..add(EnvGrantAction.serializer)
          ..add(EnvReauthenticateRequest.serializer)
          ..add(EnvRevealGrantResponse.serializer)
          ..add(ErrorResponse.serializer)
          ..add(GitCredentialListResponse.serializer)
          ..add(GitCredentialResponse.serializer)
          ..add(GitCredentialStatusRequest.serializer)
          ..add(GitRefDiscoveryResponse.serializer)
          ..add(GitRefResponse.serializer)
          ..add(InitiateUploadRequest.serializer)
          ..add(LoginRequest.serializer)
          ..add(NodeCheckResponse.serializer)
          ..add(NodeListResponse.serializer)
          ..add(NodeResponse.serializer)
          ..add(PreviewRequest.serializer)
          ..add(RefreshRequest.serializer)
          ..add(RefreshTokenPairResponse.serializer)
          ..add(RegisterApplicationEnvContent.serializer)
          ..add(RegisterApplicationEnvsRequest.serializer)
          ..add(RegisterApplicationEnvsResponse.serializer)
          ..add(ResetPasswordRequest.serializer)
          ..add(RetryApplicationEnvSyncResponse.serializer)
          ..add(RuntimeLogResponse.serializer)
          ..add(RuntimeSettings.serializer)
          ..add(SaveApplicationRequest.serializer)
          ..add(SaveSourceRequest.serializer)
          ..add(SaveTargetRequest.serializer)
          ..add(SecretFileReference.serializer)
          ..add(SessionResponse.serializer)
          ..add(SetBranchRequest.serializer)
          ..add(SetupRequest.serializer)
          ..add(SetupStatusResponse.serializer)
          ..add(SshCredentialListResponse.serializer)
          ..add(SshCredentialResponse.serializer)
          ..add(StatusResponse.serializer)
          ..add(TargetStatusRequest.serializer)
          ..add(TokenPairResponse.serializer)
          ..add(UpdateApplicationEnvRequest.serializer)
          ..add(UpdateProfileRequest.serializer)
          ..add(UpdateStatusRequest.serializer)
          ..add(UpdateUserPreferencesRequest.serializer)
          ..add(UploadStatusResponse.serializer)
          ..add(UserIdentity.serializer)
          ..add(UserListResponse.serializer)
          ..add(UserPreferencesResponse.serializer)
          ..add(UserResponse.serializer)
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(AgentReleaseResponse),
            ]),
            () => ListBuilder<AgentReleaseResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(AgentResponse)]),
            () => ListBuilder<AgentResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationEnvFileResponse),
            ]),
            () => ListBuilder<ApplicationEnvFileResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationGrantResponse),
            ]),
            () => ListBuilder<ApplicationGrantResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationResponse),
            ]),
            () => ListBuilder<ApplicationResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(AuditLogResponse)]),
            () => ListBuilder<AuditLogResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(DeploymentResponse),
            ]),
            () => ListBuilder<DeploymentResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(DeploymentTargetResponse),
            ]),
            () => ListBuilder<DeploymentTargetResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(GitCredentialResponse),
            ]),
            () => ListBuilder<GitCredentialResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(GitRefResponse)]),
            () => ListBuilder<GitRefResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(NodeResponse)]),
            () => ListBuilder<NodeResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(RegisterApplicationEnvContent),
            ]),
            () => ListBuilder<RegisterApplicationEnvContent>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(SecretFileReference),
            ]),
            () => ListBuilder<SecretFileReference>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(SecretFileReference),
            ]),
            () => ListBuilder<SecretFileReference>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(SshCredentialResponse),
            ]),
            () => ListBuilder<SshCredentialResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(DeploymentStageTaskSummary),
            ]),
            () => ListBuilder<DeploymentStageTaskSummary>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(DeploymentTargetRunResponse),
            ]),
            () => ListBuilder<DeploymentTargetRunResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(DeploymentTargetPreviewResponse),
            ]),
            () => ListBuilder<DeploymentTargetPreviewResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(UserResponse)]),
            () => ListBuilder<UserResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltMap, const [
              const FullType(String),
              const FullType.nullable(JsonObject),
            ]),
            () => MapBuilder<String, JsonObject?>(),
          ))
        .build();

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
