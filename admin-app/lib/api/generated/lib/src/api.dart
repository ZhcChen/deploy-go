//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

import 'package:dio/dio.dart';
import 'package:built_value/serializer.dart';
import 'package:deploy_go_api_client/src/serializers.dart';
import 'package:deploy_go_api_client/src/auth/basic_auth.dart';
import 'package:deploy_go_api_client/src/auth/bearer_auth.dart';
import 'package:deploy_go_api_client/src/auth/oauth.dart';
import 'package:deploy_go_api_client/src/api/agents_api.dart';
import 'package:deploy_go_api_client/src/api/agents_auth_api.dart';
import 'package:deploy_go_api_client/src/api/application_envs_api.dart';
import 'package:deploy_go_api_client/src/api/application_sources_api.dart';
import 'package:deploy_go_api_client/src/api/applications_api.dart';
import 'package:deploy_go_api_client/src/api/artifacts_http_api.dart';
import 'package:deploy_go_api_client/src/api/audit_api.dart';
import 'package:deploy_go_api_client/src/api/auth_api.dart';
import 'package:deploy_go_api_client/src/api/default_api.dart';
import 'package:deploy_go_api_client/src/api/deployment_targets_api.dart';
import 'package:deploy_go_api_client/src/api/deployments_api.dart';
import 'package:deploy_go_api_client/src/api/external_api.dart';
import 'package:deploy_go_api_client/src/api/external_keys_api.dart';
import 'package:deploy_go_api_client/src/api/git_credentials_api.dart';
import 'package:deploy_go_api_client/src/api/grants_api.dart';
import 'package:deploy_go_api_client/src/api/nodes_api.dart';
import 'package:deploy_go_api_client/src/api/runtime_logs_api.dart';
import 'package:deploy_go_api_client/src/api/settings_api.dart';
import 'package:deploy_go_api_client/src/api/ssh_credentials_api.dart';
import 'package:deploy_go_api_client/src/api/terminals_api.dart';
import 'package:deploy_go_api_client/src/api/terminals_websocket_api.dart';
import 'package:deploy_go_api_client/src/api/users_api.dart';

class DeployGoApiClient {
  static const String basePath = r'http://localhost';

  final Dio dio;
  final Serializers serializers;

  DeployGoApiClient({
    Dio? dio,
    Serializers? serializers,
    String? basePathOverride,
    List<Interceptor>? interceptors,
  })  : this.serializers = serializers ?? standardSerializers,
        this.dio = dio ??
            Dio(BaseOptions(
              baseUrl: basePathOverride ?? basePath,
              connectTimeout: const Duration(milliseconds: 5000),
              receiveTimeout: const Duration(milliseconds: 3000),
            )) {
    if (interceptors == null) {
      this.dio.interceptors.addAll([
        OAuthInterceptor(),
        BasicAuthInterceptor(),
        BearerAuthInterceptor(),
      ]);
    } else {
      this.dio.interceptors.addAll(interceptors);
    }
  }

  void setOAuthToken(String name, String token) {
    if (this.dio.interceptors.any((i) => i is OAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is OAuthInterceptor) as OAuthInterceptor).tokens[name] = token;
    }
  }

  /// Removes the OAuth token associated with the given [name].
  ///
  /// If no [OAuthInterceptor] is registered or no token exists for the given
  /// [name], this method has no effect.
  void removeOAuthToken(String name) {
    if (this.dio.interceptors.any((i) => i is OAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is OAuthInterceptor) as OAuthInterceptor).tokens.remove(name);
    }
  }

  void setBearerAuth(String name, String token) {
    if (this.dio.interceptors.any((i) => i is BearerAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is BearerAuthInterceptor) as BearerAuthInterceptor).tokens[name] = token;
    }
  }

  /// Removes the bearer authentication token associated with the given [name].
  ///
  /// If no [BearerAuthInterceptor] is registered or no token exists for the
  /// given [name], this method has no effect.
  void removeBearerAuth(String name) {
    if (this.dio.interceptors.any((i) => i is BearerAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is BearerAuthInterceptor) as BearerAuthInterceptor).tokens.remove(name);
    }
  }

  void setBasicAuth(String name, String username, String password) {
    if (this.dio.interceptors.any((i) => i is BasicAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is BasicAuthInterceptor) as BasicAuthInterceptor).authInfo[name] = BasicAuthInfo(username, password);
    }
  }

  /// Removes the basic authentication credentials associated with the given [name].
  ///
  /// If no [BasicAuthInterceptor] is registered or no credentials exist for the
  /// given [name], this method has no effect.
  void removeBasicAuth(String name) {
    if (this.dio.interceptors.any((i) => i is BasicAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is BasicAuthInterceptor) as BasicAuthInterceptor).authInfo.remove(name);
    }
  }

  /// Get AgentsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AgentsApi getAgentsApi() {
    return AgentsApi(dio, serializers);
  }

  /// Get AgentsAuthApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AgentsAuthApi getAgentsAuthApi() {
    return AgentsAuthApi(dio, serializers);
  }

  /// Get ApplicationEnvsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ApplicationEnvsApi getApplicationEnvsApi() {
    return ApplicationEnvsApi(dio, serializers);
  }

  /// Get ApplicationSourcesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ApplicationSourcesApi getApplicationSourcesApi() {
    return ApplicationSourcesApi(dio, serializers);
  }

  /// Get ApplicationsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ApplicationsApi getApplicationsApi() {
    return ApplicationsApi(dio, serializers);
  }

  /// Get ArtifactsHttpApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ArtifactsHttpApi getArtifactsHttpApi() {
    return ArtifactsHttpApi(dio, serializers);
  }

  /// Get AuditApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AuditApi getAuditApi() {
    return AuditApi(dio, serializers);
  }

  /// Get AuthApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AuthApi getAuthApi() {
    return AuthApi(dio, serializers);
  }

  /// Get DefaultApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  DefaultApi getDefaultApi() {
    return DefaultApi(dio, serializers);
  }

  /// Get DeploymentTargetsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  DeploymentTargetsApi getDeploymentTargetsApi() {
    return DeploymentTargetsApi(dio, serializers);
  }

  /// Get DeploymentsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  DeploymentsApi getDeploymentsApi() {
    return DeploymentsApi(dio, serializers);
  }

  /// Get ExternalApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ExternalApi getExternalApi() {
    return ExternalApi(dio, serializers);
  }

  /// Get ExternalKeysApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ExternalKeysApi getExternalKeysApi() {
    return ExternalKeysApi(dio, serializers);
  }

  /// Get GitCredentialsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  GitCredentialsApi getGitCredentialsApi() {
    return GitCredentialsApi(dio, serializers);
  }

  /// Get GrantsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  GrantsApi getGrantsApi() {
    return GrantsApi(dio, serializers);
  }

  /// Get NodesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  NodesApi getNodesApi() {
    return NodesApi(dio, serializers);
  }

  /// Get RuntimeLogsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  RuntimeLogsApi getRuntimeLogsApi() {
    return RuntimeLogsApi(dio, serializers);
  }

  /// Get SettingsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  SettingsApi getSettingsApi() {
    return SettingsApi(dio, serializers);
  }

  /// Get SshCredentialsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  SshCredentialsApi getSshCredentialsApi() {
    return SshCredentialsApi(dio, serializers);
  }

  /// Get TerminalsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  TerminalsApi getTerminalsApi() {
    return TerminalsApi(dio, serializers);
  }

  /// Get TerminalsWebsocketApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  TerminalsWebsocketApi getTerminalsWebsocketApi() {
    return TerminalsWebsocketApi(dio, serializers);
  }

  /// Get UsersApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  UsersApi getUsersApi() {
    return UsersApi(dio, serializers);
  }
}
