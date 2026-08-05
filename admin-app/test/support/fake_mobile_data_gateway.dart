import 'package:deploy_go_admin/api/contracts.dart';
import 'package:deploy_go_admin/api/mobile_data_gateway.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';

class FakeMobileDataGateway implements MobileDataGateway {
  FakeMobileDataGateway({
    List<ApplicationResponse>? applications,
    List<NodeResponse>? nodes,
    List<AgentResponse>? agents,
    List<UserResponse>? users,
    List<DeploymentResponse>? deployments,
    List<DeploymentTargetResponse>? deploymentTargets,
    UserIdentity? profile,
    UserPreferencesResponse? preferences,
  }) : applicationItems = applications ?? <ApplicationResponse>[],
       nodeItems = nodes ?? <NodeResponse>[],
       agentItems = agents ?? <AgentResponse>[],
       userItems = users ?? <UserResponse>[],
       deploymentItems = deployments ?? <DeploymentResponse>[],
       deploymentTargetItems =
           deploymentTargets ?? <DeploymentTargetResponse>[],
       profileValue = profile ?? fakeIdentity(),
       preferencesValue = preferences ?? fakePreferences();

  final List<ApplicationResponse> applicationItems;
  final List<NodeResponse> nodeItems;
  final List<AgentResponse> agentItems;
  final List<UserResponse> userItems;
  final List<DeploymentResponse> deploymentItems;
  final List<DeploymentTargetResponse> deploymentTargetItems;
  UserIdentity profileValue;
  UserPreferencesResponse preferencesValue;

  @override
  Future<CursorPage<ApplicationResponse>> applications({String? after}) async =>
      CursorPage(items: applicationItems);

  @override
  Future<ApplicationResponse> application(String id) async =>
      applicationItems.firstWhere((item) => item.id == id);

  @override
  Future<CursorPage<NodeResponse>> nodes({String? after}) async =>
      CursorPage(items: nodeItems);

  @override
  Future<NodeResponse> node(String id) async =>
      nodeItems.firstWhere((item) => item.id == id);

  @override
  Future<AgentResponse?> agentForNode(String nodeId) async {
    for (final agent in agentItems) {
      if (agent.nodeId == nodeId) return agent;
    }
    return null;
  }

  @override
  Future<UserIdentity> profile() async => profileValue;

  @override
  Future<UserIdentity> updateProfile(String displayName) async {
    profileValue = profileValue.rebuild(
      (builder) => builder.displayName = displayName,
    );
    return profileValue;
  }

  @override
  Future<UserPreferencesResponse> preferences() async => preferencesValue;

  @override
  Future<UserPreferencesResponse> updatePreferences(
    UserPreferencesResponse value,
  ) async {
    preferencesValue = value.rebuild(
      (builder) => builder.version = value.version + 1,
    );
    return preferencesValue;
  }

  @override
  Future<CursorPage<UserResponse>> users({String? after}) async =>
      CursorPage(items: userItems);

  @override
  Future<UserResponse> user(String id) async =>
      userItems.firstWhere((item) => item.id == id);

  @override
  Future<UserResponse> createUser({
    required String username,
    required String displayName,
    required String email,
    required String password,
  }) async {
    final user = fakeUser(
      id: 'user-${userItems.length + 1}',
      username: username,
      displayName: displayName.isEmpty ? username : displayName,
      email: email.isEmpty ? null : email,
    );
    userItems.add(user);
    return user;
  }

  @override
  Future<UserResponse> updateUserStatus(
    UserResponse user,
    String status,
  ) async {
    final updated = user.rebuild(
      (builder) => builder
        ..status = status
        ..version = user.version + 1,
    );
    final index = userItems.indexWhere((item) => item.id == user.id);
    if (index >= 0) userItems[index] = updated;
    return updated;
  }

  @override
  Future<CursorPage<DeploymentResponse>> deployments({String? after}) async =>
      CursorPage(items: deploymentItems);

  @override
  Future<DeploymentResponse> deployment(String id) async =>
      deploymentItems.firstWhere((item) => item.id == id);

  @override
  Future<List<DeploymentTargetResponse>> deploymentTargets(
    String applicationId,
  ) async => deploymentTargetItems
      .where((item) => item.applicationId == applicationId)
      .toList(growable: false);

  @override
  Future<DeploymentPreviewResponse> previewDeployment(
    String targetId,
    Map<String, Object?> parameters,
  ) async {
    final target = deploymentTargetItems.firstWhere(
      (item) => item.id == targetId,
    );
    return fakeDeploymentPreview(target: target);
  }

  @override
  Future<DeploymentResponse> confirmDeployment({
    required DeploymentPreviewResponse preview,
    required Map<String, Object?> parameters,
    required String idempotencyKey,
  }) async {
    final deployment = fakeDeployment(
      id: 'deployment-${deploymentItems.length + 1}',
      targetId: preview.targetId,
    );
    deploymentItems.add(deployment);
    return deployment;
  }

  @override
  Future<DeploymentResponse> cancelDeployment(String id) async {
    final index = deploymentItems.indexWhere((item) => item.id == id);
    final saved = deploymentItems[index].rebuild(
      (builder) => builder
        ..status = 'canceled'
        ..phase = 'canceled'
        ..version = deploymentItems[index].version + 1,
    );
    deploymentItems[index] = saved;
    return saved;
  }

  @override
  Future<DeploymentResponse> retryDeployment(
    String id,
    String idempotencyKey,
  ) async {
    final source = deploymentItems.firstWhere((item) => item.id == id);
    final deployment = fakeDeployment(
      id: 'deployment-${deploymentItems.length + 1}',
      targetId: source.targetId,
    );
    deploymentItems.add(deployment);
    return deployment;
  }
}

UserIdentity fakeIdentity({String identity = 'administrator'}) => UserIdentity(
  (builder) => builder
    ..id = identity == 'administrator' ? 'admin-1' : 'user-1'
    ..username = identity == 'administrator' ? 'admin' : 'operator'
    ..displayName = identity == 'administrator' ? '管理员' : '部署用户'
    ..identity = identity,
);

ApplicationResponse fakeApplication({
  String id = 'app-1',
  String name = '示例应用',
}) => ApplicationResponse(
  (builder) => builder
    ..id = id
    ..name = name
    ..slug = id
    ..description = '用于测试的应用'
    ..status = 'active'
    ..version = 1
    ..createdAt = '2026-08-02T00:00:00Z'
    ..updatedAt = '2026-08-02T00:00:00Z',
);

AgentResponse fakeAgent({
  String id = 'agent-1',
  String nodeId = 'node-1',
  String environment = 'prod',
  String status = 'online',
  String? version = '0.1.0',
  String? hostname = 'node-1',
  String? architecture = 'x86_64',
  String? lastSeenAt = '2026-08-03T00:00:00Z',
}) => AgentResponse(
  (builder) => builder
    ..id = id
    ..nodeId = nodeId
    ..name = '节点 Agent'
    ..environment = environment
    ..status = status
    ..agentVersion = version
    ..hostname = hostname
    ..architecture = architecture
    ..lastSeenAt = lastSeenAt
    ..createdAt = '2026-08-02T00:00:00Z',
);

UserPreferencesResponse fakePreferences() => UserPreferencesResponse(
  (builder) => builder
    ..notifyDeploymentFailed = true
    ..notifyDeploymentCompleted = false
    ..notifyNodeUnhealthy = true
    ..timeFormat = '24h'
    ..followLogs = true
    ..version = 1,
);

UserResponse fakeUser({
  required String id,
  required String username,
  required String displayName,
  String? email,
  String identity = 'user',
  String status = 'active',
}) => UserResponse(
  (builder) => builder
    ..id = id
    ..username = username
    ..displayName = displayName
    ..email = email
    ..identity = identity
    ..status = status
    ..version = 1,
);

DeploymentTargetResponse fakeDeploymentTarget({
  String id = 'target-1',
  String applicationId = 'app-1',
}) => DeploymentTargetResponse(
  (builder) => builder
    ..id = id
    ..applicationId = applicationId
    ..nodeId = 'node-1'
    ..environment = 'production'
    ..scriptPath = 'deploy/release.sh'
    ..timeoutSeconds = 600
    ..status = 'active'
    ..snapshotHash = 'snapshot-target'
    ..secretFileReferences.replace(const [])
    ..version = 1
    ..createdAt = '2026-08-02T00:00:00Z'
    ..updatedAt = '2026-08-02T00:00:00Z',
);

DeploymentPreviewResponse fakeDeploymentPreview({
  required DeploymentTargetResponse target,
}) => DeploymentPreviewResponse(
  (builder) => builder
    ..targetId = target.id
    ..applicationId = target.applicationId
    ..applicationName = '示例应用'
    ..nodeId = target.nodeId
    ..nodeName = '示例节点'
    ..environment = target.environment
    ..scriptPath = target.scriptPath
    ..snapshotHash = target.snapshotHash,
);

DeploymentResponse fakeDeployment({
  required String id,
  String targetId = 'target-1',
  String status = 'running',
}) => DeploymentResponse(
  (builder) => builder
    ..id = id
    ..targetId = targetId
    ..requestedBy = 'admin-1'
    ..status = status
    ..phase = status
    ..snapshotHash = 'snapshot-deployment'
    ..protocolComplete = status == 'succeeded'
    ..version = 1
    ..createdAt = '2026-08-02T00:00:00Z'
    ..updatedAt = '2026-08-02T00:00:00Z'
    ..queuedAt = '2026-08-02T00:00:00Z',
);
