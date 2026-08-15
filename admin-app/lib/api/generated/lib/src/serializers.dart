//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_import

import 'package:one_of_serializer/any_of_serializer.dart';
import 'package:one_of_serializer/one_of_serializer.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/serializer.dart';
import 'package:built_value/standard_json_plugin.dart';
import 'package:built_value/iso_8601_date_time_serializer.dart';
import 'package:deploy_go_api_client/src/date_serializer.dart';
import 'package:deploy_go_api_client/src/model/date.dart';

import 'package:deploy_go_api_client/src/model/agent_enrollment_response.dart';
import 'package:deploy_go_api_client/src/model/agent_install_command_response.dart';
import 'package:deploy_go_api_client/src/model/agent_list_response.dart';
import 'package:deploy_go_api_client/src/model/agent_release_list_response.dart';
import 'package:deploy_go_api_client/src/model/agent_release_response.dart';
import 'package:deploy_go_api_client/src/model/agent_response.dart';
import 'package:deploy_go_api_client/src/model/application_deployment_preview_response.dart';
import 'package:deploy_go_api_client/src/model/application_env_file_list_response.dart';
import 'package:deploy_go_api_client/src/model/application_env_file_response.dart';
import 'package:deploy_go_api_client/src/model/application_env_plaintext_response.dart';
import 'package:deploy_go_api_client/src/model/application_env_registration_response.dart';
import 'package:deploy_go_api_client/src/model/application_env_sync_response.dart';
import 'package:deploy_go_api_client/src/model/application_grant_list_response.dart';
import 'package:deploy_go_api_client/src/model/application_grant_response.dart';
import 'package:deploy_go_api_client/src/model/application_list_response.dart';
import 'package:deploy_go_api_client/src/model/application_response.dart';
import 'package:deploy_go_api_client/src/model/application_source_response.dart';
import 'package:deploy_go_api_client/src/model/application_status_request.dart';
import 'package:deploy_go_api_client/src/model/audit_log_list_response.dart';
import 'package:deploy_go_api_client/src/model/audit_log_response.dart';
import 'package:deploy_go_api_client/src/model/confirm_request.dart';
import 'package:deploy_go_api_client/src/model/create_agent_request.dart';
import 'package:deploy_go_api_client/src/model/create_external_api_key_request.dart';
import 'package:deploy_go_api_client/src/model/create_git_credential_request.dart';
import 'package:deploy_go_api_client/src/model/create_user_request.dart';
import 'package:deploy_go_api_client/src/model/csrf_token_response.dart';
import 'package:deploy_go_api_client/src/model/delete_application_env_request.dart';
import 'package:deploy_go_api_client/src/model/deployment_event_list_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_event_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_list_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_log_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_preview_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_stage_task_summary.dart';
import 'package:deploy_go_api_client/src/model/deployment_target_list_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_target_preview_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_target_response.dart';
import 'package:deploy_go_api_client/src/model/deployment_target_run_response.dart';
import 'package:deploy_go_api_client/src/model/enroll_request.dart';
import 'package:deploy_go_api_client/src/model/env_grant_action.dart';
import 'package:deploy_go_api_client/src/model/env_reauthenticate_request.dart';
import 'package:deploy_go_api_client/src/model/env_reveal_grant_response.dart';
import 'package:deploy_go_api_client/src/model/error_response.dart';
import 'package:deploy_go_api_client/src/model/external_api_key_created_response.dart';
import 'package:deploy_go_api_client/src/model/external_api_key_list_response.dart';
import 'package:deploy_go_api_client/src/model/external_api_key_summary.dart';
import 'package:deploy_go_api_client/src/model/git_credential_list_response.dart';
import 'package:deploy_go_api_client/src/model/git_credential_response.dart';
import 'package:deploy_go_api_client/src/model/git_credential_status_request.dart';
import 'package:deploy_go_api_client/src/model/git_ref_discovery_response.dart';
import 'package:deploy_go_api_client/src/model/git_ref_response.dart';
import 'package:deploy_go_api_client/src/model/image_deploy_spec.dart';
import 'package:deploy_go_api_client/src/model/image_template.dart';
import 'package:deploy_go_api_client/src/model/initiate_upload_request.dart';
import 'package:deploy_go_api_client/src/model/login_request.dart';
import 'package:deploy_go_api_client/src/model/node_check_response.dart';
import 'package:deploy_go_api_client/src/model/node_list_response.dart';
import 'package:deploy_go_api_client/src/model/node_response.dart';
import 'package:deploy_go_api_client/src/model/preview_request.dart';
import 'package:deploy_go_api_client/src/model/refresh_request.dart';
import 'package:deploy_go_api_client/src/model/refresh_token_pair_response.dart';
import 'package:deploy_go_api_client/src/model/register_admin_application_env_content.dart';
import 'package:deploy_go_api_client/src/model/register_admin_application_envs_request.dart';
import 'package:deploy_go_api_client/src/model/register_application_env_content.dart';
import 'package:deploy_go_api_client/src/model/register_application_envs_request.dart';
import 'package:deploy_go_api_client/src/model/register_application_envs_response.dart';
import 'package:deploy_go_api_client/src/model/reset_password_request.dart';
import 'package:deploy_go_api_client/src/model/retry_application_env_sync_response.dart';
import 'package:deploy_go_api_client/src/model/runtime_log_response.dart';
import 'package:deploy_go_api_client/src/model/runtime_settings.dart';
import 'package:deploy_go_api_client/src/model/save_application_request.dart';
import 'package:deploy_go_api_client/src/model/save_source_request.dart';
import 'package:deploy_go_api_client/src/model/save_target_request.dart';
import 'package:deploy_go_api_client/src/model/secret_file_reference.dart';
import 'package:deploy_go_api_client/src/model/session_response.dart';
import 'package:deploy_go_api_client/src/model/set_branch_request.dart';
import 'package:deploy_go_api_client/src/model/setup_request.dart';
import 'package:deploy_go_api_client/src/model/setup_status_response.dart';
import 'package:deploy_go_api_client/src/model/ssh_credential_list_response.dart';
import 'package:deploy_go_api_client/src/model/ssh_credential_response.dart';
import 'package:deploy_go_api_client/src/model/status_response.dart';
import 'package:deploy_go_api_client/src/model/target_status_request.dart';
import 'package:deploy_go_api_client/src/model/terminal_capability_response.dart';
import 'package:deploy_go_api_client/src/model/terminal_session_response.dart';
import 'package:deploy_go_api_client/src/model/token_pair_response.dart';
import 'package:deploy_go_api_client/src/model/update_application_env_request.dart';
import 'package:deploy_go_api_client/src/model/update_external_api_key_applications_request.dart';
import 'package:deploy_go_api_client/src/model/update_profile_request.dart';
import 'package:deploy_go_api_client/src/model/update_status_request.dart';
import 'package:deploy_go_api_client/src/model/update_user_preferences_request.dart';
import 'package:deploy_go_api_client/src/model/upload_status_response.dart';
import 'package:deploy_go_api_client/src/model/user_identity.dart';
import 'package:deploy_go_api_client/src/model/user_list_response.dart';
import 'package:deploy_go_api_client/src/model/user_preferences_response.dart';
import 'package:deploy_go_api_client/src/model/user_response.dart';

part 'serializers.g.dart';

@SerializersFor([
  AgentEnrollmentResponse,
  AgentInstallCommandResponse,
  AgentListResponse,
  AgentReleaseListResponse,
  AgentReleaseResponse,
  AgentResponse,
  ApplicationDeploymentPreviewResponse,
  ApplicationEnvFileListResponse,
  ApplicationEnvFileResponse,
  ApplicationEnvPlaintextResponse,
  ApplicationEnvRegistrationResponse,
  ApplicationEnvSyncResponse,
  ApplicationGrantListResponse,
  ApplicationGrantResponse,
  ApplicationListResponse,
  ApplicationResponse,
  ApplicationSourceResponse,
  ApplicationStatusRequest,
  AuditLogListResponse,
  AuditLogResponse,
  ConfirmRequest,
  CreateAgentRequest,
  CreateExternalApiKeyRequest,
  CreateGitCredentialRequest,
  CreateUserRequest,
  CsrfTokenResponse,
  DeleteApplicationEnvRequest,
  DeploymentEventListResponse,
  DeploymentEventResponse,
  DeploymentListResponse,
  DeploymentLogResponse,
  DeploymentPreviewResponse,
  DeploymentResponse,
  DeploymentStageTaskSummary,
  DeploymentTargetListResponse,
  DeploymentTargetPreviewResponse,
  DeploymentTargetResponse,
  DeploymentTargetRunResponse,
  EnrollRequest,
  EnvGrantAction,
  EnvReauthenticateRequest,
  EnvRevealGrantResponse,
  ErrorResponse,
  ExternalApiKeyCreatedResponse,
  ExternalApiKeyListResponse,
  ExternalApiKeySummary,
  GitCredentialListResponse,
  GitCredentialResponse,
  GitCredentialStatusRequest,
  GitRefDiscoveryResponse,
  GitRefResponse,
  ImageDeploySpec,
  ImageTemplate,
  InitiateUploadRequest,
  LoginRequest,
  NodeCheckResponse,
  NodeListResponse,
  NodeResponse,
  PreviewRequest,
  RefreshRequest,
  RefreshTokenPairResponse,
  RegisterAdminApplicationEnvContent,
  RegisterAdminApplicationEnvsRequest,
  RegisterApplicationEnvContent,
  RegisterApplicationEnvsRequest,
  RegisterApplicationEnvsResponse,
  ResetPasswordRequest,
  RetryApplicationEnvSyncResponse,
  RuntimeLogResponse,
  RuntimeSettings,
  SaveApplicationRequest,
  SaveSourceRequest,
  SaveTargetRequest,
  SecretFileReference,
  SessionResponse,
  SetBranchRequest,
  SetupRequest,
  SetupStatusResponse,
  SshCredentialListResponse,
  SshCredentialResponse,
  StatusResponse,
  TargetStatusRequest,
  TerminalCapabilityResponse,
  TerminalSessionResponse,
  TokenPairResponse,
  UpdateApplicationEnvRequest,
  UpdateExternalApiKeyApplicationsRequest,
  UpdateProfileRequest,
  UpdateStatusRequest,
  UpdateUserPreferencesRequest,
  UploadStatusResponse,
  UserIdentity,
  UserListResponse,
  UserPreferencesResponse,
  UserResponse,
])
Serializers serializers = (_$serializers.toBuilder()
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(DeploymentTargetPreviewResponse)]),
        () => ListBuilder<DeploymentTargetPreviewResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(RegisterAdminApplicationEnvContent)]),
        () => ListBuilder<RegisterAdminApplicationEnvContent>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(DeploymentEventResponse)]),
        () => ListBuilder<DeploymentEventResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(AuditLogResponse)]),
        () => ListBuilder<AuditLogResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(AgentResponse)]),
        () => ListBuilder<AgentResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(GitCredentialResponse)]),
        () => ListBuilder<GitCredentialResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(NodeResponse)]),
        () => ListBuilder<NodeResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ApplicationResponse)]),
        () => ListBuilder<ApplicationResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(DeploymentStageTaskSummary)]),
        () => ListBuilder<DeploymentStageTaskSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(DeploymentTargetResponse)]),
        () => ListBuilder<DeploymentTargetResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(UserResponse)]),
        () => ListBuilder<UserResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(DeploymentResponse)]),
        () => ListBuilder<DeploymentResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ApplicationEnvSyncResponse)]),
        () => ListBuilder<ApplicationEnvSyncResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ApplicationGrantResponse)]),
        () => ListBuilder<ApplicationGrantResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ApplicationEnvFileResponse)]),
        () => ListBuilder<ApplicationEnvFileResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SecretFileReference)]),
        () => ListBuilder<SecretFileReference>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(RegisterApplicationEnvContent)]),
        () => ListBuilder<RegisterApplicationEnvContent>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SshCredentialResponse)]),
        () => ListBuilder<SshCredentialResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(DeploymentTargetRunResponse)]),
        () => ListBuilder<DeploymentTargetRunResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(GitRefResponse)]),
        () => ListBuilder<GitRefResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(int)]),
        () => ListBuilder<int>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ExternalApiKeySummary)]),
        () => ListBuilder<ExternalApiKeySummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
        () => MapBuilder<String, JsonObject?>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(String)]),
        () => ListBuilder<String>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(AgentReleaseResponse)]),
        () => ListBuilder<AgentReleaseResponse>(),
      )
      ..add(const OneOfSerializer())
      ..add(const AnyOfSerializer())
      ..add(const DateSerializer())
      ..add(Iso8601DateTimeSerializer())
    ).build();

Serializers standardSerializers =
    (serializers.toBuilder()..addPlugin(StandardJsonPlugin())).build();
