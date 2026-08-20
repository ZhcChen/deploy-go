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
          ..add(ApplicationConfigDiffResponse.serializer)
          ..add(ApplicationConfigFileListResponse.serializer)
          ..add(ApplicationConfigFileResponse.serializer)
          ..add(ApplicationConfigValidationResponse.serializer)
          ..add(ApplicationConfigVersionListResponse.serializer)
          ..add(ApplicationConfigVersionResponse.serializer)
          ..add(ApplicationDeploymentPreviewResponse.serializer)
          ..add(ApplicationEnvFileListResponse.serializer)
          ..add(ApplicationEnvFileResponse.serializer)
          ..add(ApplicationEnvPlaintextResponse.serializer)
          ..add(ApplicationEnvRegistrationResponse.serializer)
          ..add(ApplicationEnvSyncResponse.serializer)
          ..add(ApplicationGrantListResponse.serializer)
          ..add(ApplicationGrantResponse.serializer)
          ..add(ApplicationListResponse.serializer)
          ..add(ApplicationResponse.serializer)
          ..add(ApplicationSourceResponse.serializer)
          ..add(ApplicationStatusRequest.serializer)
          ..add(ApplicationTemplateFileResponse.serializer)
          ..add(ApplicationTemplateListResponse.serializer)
          ..add(ApplicationTemplateResponse.serializer)
          ..add(AuditLogListResponse.serializer)
          ..add(AuditLogResponse.serializer)
          ..add(ConfigDiagnostic.serializer)
          ..add(ConfigDiffQuery.serializer)
          ..add(ConfigGrantAction.serializer)
          ..add(ConfigReauthenticateRequest.serializer)
          ..add(ConfigRevealGrantResponse.serializer)
          ..add(ConfirmRequest.serializer)
          ..add(ControlledPatchRequest.serializer)
          ..add(CreateAgentRequest.serializer)
          ..add(CreateExternalApiKeyRequest.serializer)
          ..add(CreateGitCredentialRequest.serializer)
          ..add(CreateUserRequest.serializer)
          ..add(CsrfTokenResponse.serializer)
          ..add(DeleteApplicationConfigWorkspaceRequest.serializer)
          ..add(DeleteApplicationEnvRequest.serializer)
          ..add(DeletePlatformConfigurationCenterRequest.serializer)
          ..add(DeploymentEventListResponse.serializer)
          ..add(DeploymentEventResponse.serializer)
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
          ..add(ExternalApiKeyCreatedResponse.serializer)
          ..add(ExternalApiKeyListResponse.serializer)
          ..add(ExternalApiKeySummary.serializer)
          ..add(GenerateSecretRequest.serializer)
          ..add(GenerateSecretResponse.serializer)
          ..add(GitCredentialListResponse.serializer)
          ..add(GitCredentialResponse.serializer)
          ..add(GitCredentialStatusRequest.serializer)
          ..add(GitRefDiscoveryResponse.serializer)
          ..add(GitRefResponse.serializer)
          ..add(HistoryPoint.serializer)
          ..add(ImageDeploySpec.serializer)
          ..add(ImageTemplate.serializer)
          ..add(InitializeApplicationConfigsRequest.serializer)
          ..add(InitializeApplicationConfigsResponse.serializer)
          ..add(InitiateUploadRequest.serializer)
          ..add(LatestTelemetry.serializer)
          ..add(LoginRequest.serializer)
          ..add(MetricValue.serializer)
          ..add(NodeCheckResponse.serializer)
          ..add(NodeListResponse.serializer)
          ..add(NodeResponse.serializer)
          ..add(PlatformConfigurationCenterResponse.serializer)
          ..add(PreviewRequest.serializer)
          ..add(RefreshRequest.serializer)
          ..add(RefreshTokenPairResponse.serializer)
          ..add(RegisterAdminApplicationEnvContent.serializer)
          ..add(RegisterAdminApplicationEnvsRequest.serializer)
          ..add(RegisterApplicationEnvContent.serializer)
          ..add(RegisterApplicationEnvsRequest.serializer)
          ..add(RegisterApplicationEnvsResponse.serializer)
          ..add(ResetPasswordRequest.serializer)
          ..add(RestoreApplicationConfigRequest.serializer)
          ..add(RetryApplicationEnvSyncResponse.serializer)
          ..add(RuntimeLogResponse.serializer)
          ..add(RuntimeSettings.serializer)
          ..add(SaveApplicationRequest.serializer)
          ..add(SavePlatformConfigurationCenterRequest.serializer)
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
          ..add(TelemetryResponse.serializer)
          ..add(TerminalCapabilityResponse.serializer)
          ..add(TerminalSessionResponse.serializer)
          ..add(TokenPairResponse.serializer)
          ..add(UpdateApplicationConfigRequest.serializer)
          ..add(UpdateApplicationEnvRequest.serializer)
          ..add(UpdateExternalApiKeyApplicationsRequest.serializer)
          ..add(UpdateProfileRequest.serializer)
          ..add(UpdateStatusRequest.serializer)
          ..add(UpdateUserPreferencesRequest.serializer)
          ..add(UploadStatusResponse.serializer)
          ..add(UserIdentity.serializer)
          ..add(UserListResponse.serializer)
          ..add(UserPreferencesResponse.serializer)
          ..add(UserResponse.serializer)
          ..add(ValidateApplicationConfigRequest.serializer)
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
              const FullType(ApplicationConfigFileResponse),
            ]),
            () => ListBuilder<ApplicationConfigFileResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationConfigVersionResponse),
            ]),
            () => ListBuilder<ApplicationConfigVersionResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationEnvFileResponse),
            ]),
            () => ListBuilder<ApplicationEnvFileResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationEnvSyncResponse),
            ]),
            () => ListBuilder<ApplicationEnvSyncResponse>(),
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
            const FullType(BuiltList, const [
              const FullType(ApplicationTemplateFileResponse),
            ]),
            () => ListBuilder<ApplicationTemplateFileResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(ApplicationTemplateResponse),
            ]),
            () => ListBuilder<ApplicationTemplateResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(AuditLogResponse)]),
            () => ListBuilder<AuditLogResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(ConfigDiagnostic)]),
            () => ListBuilder<ConfigDiagnostic>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(DeploymentEventResponse),
            ]),
            () => ListBuilder<DeploymentEventResponse>(),
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
              const FullType(ExternalApiKeySummary),
            ]),
            () => ListBuilder<ExternalApiKeySummary>(),
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
            const FullType(BuiltList, const [const FullType(HistoryPoint)]),
            () => ListBuilder<HistoryPoint>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [const FullType(NodeResponse)]),
            () => ListBuilder<NodeResponse>(),
          )
          ..addBuilderFactory(
            const FullType(BuiltList, const [
              const FullType(RegisterAdminApplicationEnvContent),
            ]),
            () => ListBuilder<RegisterAdminApplicationEnvContent>(),
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
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
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
            const FullType(BuiltList, const [const FullType(String)]),
            () => ListBuilder<String>(),
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
