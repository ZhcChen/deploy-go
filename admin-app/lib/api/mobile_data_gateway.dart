import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:dio/dio.dart';
import 'package:built_value/json_object.dart';

import '../security/secure_session_store.dart';
import 'contracts.dart';
import 'deploy_go_api.dart';

abstract interface class MobileDataGateway {
  Future<CursorPage<ApplicationResponse>> applications({String? after});
  Future<ApplicationResponse> application(String id);
  Future<CursorPage<NodeResponse>> nodes({String? after});
  Future<NodeResponse> node(String id);
  Future<AgentResponse?> agentForNode(String nodeId);
  Future<UserIdentity> profile();
  Future<UserIdentity> updateProfile(String displayName);
  Future<UserPreferencesResponse> preferences();
  Future<UserPreferencesResponse> updatePreferences(
    UserPreferencesResponse value,
  );
  Future<CursorPage<UserResponse>> users({String? after});
  Future<UserResponse> user(String id);
  Future<UserResponse> createUser({
    required String username,
    required String displayName,
    required String email,
    required String password,
  });
  Future<UserResponse> updateUserStatus(UserResponse user, String status);
  Future<CursorPage<DeploymentResponse>> deployments({String? after});
  Future<DeploymentResponse> deployment(String id);
  Future<List<DeploymentTargetResponse>> deploymentTargets(
    String applicationId,
  );
  Future<DeploymentPreviewResponse> previewDeployment(
    String targetId,
    Map<String, Object?> parameters,
  );
  Future<DeploymentResponse> confirmDeployment({
    required DeploymentPreviewResponse preview,
    required Map<String, Object?> parameters,
    required String idempotencyKey,
  });
  Future<DeploymentResponse> cancelDeployment(String id);
  Future<DeploymentResponse> retryDeployment(String id, String idempotencyKey);
}

class DeployGoMobileDataGateway implements MobileDataGateway {
  DeployGoMobileDataGateway(this._api, this._sessionStore);

  final DeployGoApi _api;
  final SecureSessionStore _sessionStore;

  @override
  Future<CursorPage<ApplicationResponse>> applications({String? after}) =>
      _guard(() async {
        final data = _required(
          (await _api.client.getApplicationsApi().applicationsList(
            limit: 50,
            after: after,
          )).data,
        );
        return CursorPage(
          items: data.items.toList(),
          nextCursor: data.nextCursor,
        );
      });

  @override
  Future<ApplicationResponse> application(String id) => _guard(
    () async => _required(
      (await _api.client.getApplicationsApi().applicationsShow(id: id)).data,
    ),
  );

  @override
  Future<CursorPage<NodeResponse>> nodes({String? after}) => _guard(() async {
    final data = _required(
      (await _api.client.getNodesApi().nodesList(limit: 50, after: after)).data,
    );
    return CursorPage(items: data.items.toList(), nextCursor: data.nextCursor);
  });

  @override
  Future<NodeResponse> node(String id) => _guard(
    () async =>
        _required((await _api.client.getNodesApi().nodesShow(id: id)).data),
  );

  @override
  Future<AgentResponse?> agentForNode(String nodeId) => _guard(() async {
    String? after;
    do {
      final data = _required(
        (await _api.client.getAgentsApi().agentsList(
          limit: 200,
          after: after,
        )).data,
      );
      for (final agent in data.items) {
        if (agent.nodeId == nodeId) return agent;
      }
      after = data.nextCursor;
    } while (after != null);
    return null;
  });

  @override
  Future<UserIdentity> profile() => _guard(
    () async => _required((await _api.client.getAuthApi().authProfile()).data),
  );

  @override
  Future<UserIdentity> updateProfile(String displayName) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getAuthApi().authUpdateProfile(
        xCSRFToken: csrf,
        updateProfileRequest: UpdateProfileRequest(
          (builder) => builder.displayName = displayName,
        ),
      )).data,
    );
  });

  @override
  Future<UserPreferencesResponse> preferences() => _guard(
    () async =>
        _required((await _api.client.getAuthApi().authPreferences()).data),
  );

  @override
  Future<UserPreferencesResponse> updatePreferences(
    UserPreferencesResponse value,
  ) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getAuthApi().authUpdatePreferences(
        xCSRFToken: csrf,
        updateUserPreferencesRequest: UpdateUserPreferencesRequest(
          (builder) => builder
            ..notifyDeploymentFailed = value.notifyDeploymentFailed
            ..notifyDeploymentCompleted = value.notifyDeploymentCompleted
            ..notifyNodeUnhealthy = value.notifyNodeUnhealthy
            ..timeFormat = value.timeFormat
            ..followLogs = value.followLogs
            ..version = value.version,
        ),
      )).data,
    );
  });

  @override
  Future<CursorPage<UserResponse>> users({String? after}) => _guard(() async {
    final data = _required(
      (await _api.client.getUsersApi().usersList(limit: 50, after: after)).data,
    );
    return CursorPage(items: data.items.toList(), nextCursor: data.nextCursor);
  });

  @override
  Future<UserResponse> user(String id) => _guard(
    () async =>
        _required((await _api.client.getUsersApi().usersShow(id: id)).data),
  );

  @override
  Future<UserResponse> createUser({
    required String username,
    required String displayName,
    required String email,
    required String password,
  }) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getUsersApi().usersCreate(
        xCSRFToken: csrf,
        createUserRequest: CreateUserRequest(
          (builder) => builder
            ..username = username
            ..password = password
            ..displayName = displayName.isEmpty ? null : displayName
            ..email = email.isEmpty ? null : email,
        ),
      )).data,
    );
  });

  @override
  Future<UserResponse> updateUserStatus(UserResponse user, String status) =>
      _guard(() async {
        final csrf = await _csrf();
        return _required(
          (await _api.client.getUsersApi().usersUpdateStatus(
            id: user.id,
            xCSRFToken: csrf,
            updateStatusRequest: UpdateStatusRequest(
              (builder) => builder
                ..status = status
                ..version = user.version,
            ),
          )).data,
        );
      });

  @override
  Future<CursorPage<DeploymentResponse>> deployments({String? after}) =>
      _guard(() async {
        final data = _required(
          (await _api.client.getDeploymentsApi().deploymentsList(
            limit: 30,
            after: after,
          )).data,
        );
        return CursorPage(
          items: data.items.toList(),
          nextCursor: data.nextCursor,
        );
      });

  @override
  Future<DeploymentResponse> deployment(String id) => _guard(
    () async => _required(
      (await _api.client.getDeploymentsApi().deploymentsShow(id: id)).data,
    ),
  );

  @override
  Future<List<DeploymentTargetResponse>> deploymentTargets(
    String applicationId,
  ) => _guard(() async {
    final data = _required(
      (await _api.client.getDeploymentTargetsApi().deploymentTargetsList(
        applicationId: applicationId,
        limit: 100,
      )).data,
    );
    return data.items.toList(growable: false);
  });

  @override
  Future<DeploymentPreviewResponse> previewDeployment(
    String targetId,
    Map<String, Object?> parameters,
  ) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getDeploymentsApi().deploymentsPreview(
        id: targetId,
        xCSRFToken: csrf,
        previewRequest: PreviewRequest(
          (builder) => builder.parameters = JsonObject(parameters),
        ),
      )).data,
    );
  });

  @override
  Future<DeploymentResponse> confirmDeployment({
    required DeploymentPreviewResponse preview,
    required Map<String, Object?> parameters,
    required String idempotencyKey,
  }) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getDeploymentsApi().deploymentsConfirm(
        id: preview.targetId,
        xCSRFToken: csrf,
        headers: <String, dynamic>{'Idempotency-Key': idempotencyKey},
        confirmRequest: ConfirmRequest(
          (builder) => builder
            ..snapshotHash = preview.snapshotHash
            ..parameters = JsonObject(parameters),
        ),
      )).data,
    );
  });

  @override
  Future<DeploymentResponse> cancelDeployment(String id) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getDeploymentsApi().deploymentsCancel(
        id: id,
        xCSRFToken: csrf,
      )).data,
    );
  });

  @override
  Future<DeploymentResponse> retryDeployment(
    String id,
    String idempotencyKey,
  ) => _guard(() async {
    final csrf = await _csrf();
    return _required(
      (await _api.client.getDeploymentsApi().deploymentsRetry(
        id: id,
        xCSRFToken: csrf,
        headers: <String, dynamic>{'Idempotency-Key': idempotencyKey},
      )).data,
    );
  });

  Future<String> _csrf() async {
    final token = await _sessionStore.readCsrfToken();
    if (token == null || token.isEmpty) {
      throw const ApiFailureException(
        ApiFailure(
          status: 401,
          code: 'missing_csrf_token',
          message: '会话安全信息已失效，请重新登录',
          requestId: '',
        ),
      );
    }
    return token;
  }
}

Future<T> _guard<T>(Future<T> Function() action) async {
  try {
    return await action();
  } on DioException catch (error) {
    throw ApiFailureException.fromDio(error);
  }
}

T _required<T>(T? value) {
  if (value == null) {
    throw const ApiFailureException(
      ApiFailure(
        status: 502,
        code: 'invalid_response',
        message: '服务返回缺少必要数据',
        requestId: '',
      ),
    );
  }
  return value;
}

class ApiFailureException implements Exception {
  const ApiFailureException(this.failure);

  factory ApiFailureException.fromDio(DioException error) {
    final data = error.response?.data;
    final map = data is Map ? data : const <Object?, Object?>{};
    return ApiFailureException(
      ApiFailure(
        status: error.response?.statusCode ?? 0,
        code: map['code']?.toString() ?? 'network_error',
        message: map['message']?.toString() ?? '无法连接部署控制服务',
        requestId: map['request_id']?.toString() ?? '',
        details: map['details'],
      ),
    );
  }

  final ApiFailure failure;

  @override
  String toString() => failure.message;
}
