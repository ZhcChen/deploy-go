import 'package:deploy_go_admin/api/contracts.dart';
import 'package:deploy_go_admin/api/mobile_data_gateway.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';

class FakeMobileDataGateway implements MobileDataGateway {
  FakeMobileDataGateway({
    List<ApplicationResponse>? applications,
    List<NodeResponse>? nodes,
    List<UserResponse>? users,
    UserIdentity? profile,
    UserPreferencesResponse? preferences,
  }) : applicationItems = applications ?? <ApplicationResponse>[],
       nodeItems = nodes ?? <NodeResponse>[],
       userItems = users ?? <UserResponse>[],
       profileValue = profile ?? fakeIdentity(),
       preferencesValue = preferences ?? fakePreferences();

  final List<ApplicationResponse> applicationItems;
  final List<NodeResponse> nodeItems;
  final List<UserResponse> userItems;
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
}

UserIdentity fakeIdentity({String identity = 'administrator'}) => UserIdentity(
  (builder) => builder
    ..id = identity == 'administrator' ? 'admin-1' : 'user-1'
    ..username = identity == 'administrator' ? 'admin' : 'operator'
    ..displayName = identity == 'administrator' ? '管理员' : '部署用户'
    ..identity = identity,
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
