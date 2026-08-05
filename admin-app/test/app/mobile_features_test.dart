import 'dart:async';

import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/api/contracts.dart';
import 'package:deploy_go_admin/api/mobile_data_gateway.dart';
import 'package:deploy_go_admin/app/deploy_go_app.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import '../support/fake_mobile_data_gateway.dart';

void main() {
  testWidgets('管理员可进入用户管理且没有角色、邀请或注册入口', (tester) async {
    final auth = _FeatureAuthGateway(identity: 'administrator');
    final data = FakeMobileDataGateway(
      users: <UserResponse>[
        fakeUser(id: 'user-1', username: 'operator', displayName: '部署用户'),
      ],
    );
    await _pump(tester, auth, data);

    await tester.tap(find.text('我的'));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey<String>('profile-identity-header')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey<String>('user-management-entry')),
      findsOneWidget,
    );
    expect(find.textContaining('角色'), findsNothing);
    expect(find.textContaining('邀请'), findsNothing);
    expect(find.textContaining('注册'), findsNothing);

    await tester.tap(
      find.byKey(const ValueKey<String>('user-management-entry')),
    );
    await tester.pumpAndSettle();
    expect(find.text('部署用户'), findsOneWidget);
    await tester.tap(find.byTooltip('新增用户'));
    await tester.pumpAndSettle();
    expect(find.text('创建普通用户'), findsOneWidget);
    await tester.enterText(
      find.byKey(const ValueKey<String>('new-user-username')),
      'release',
    );
    await tester.enterText(
      find.byKey(const ValueKey<String>('new-user-password')),
      'initial-pass-123',
    );
    final create = find.text('创建普通用户');
    await tester.ensureVisible(create);
    await tester.tap(create);
    await tester.pumpAndSettle();
    expect(data.userItems.map((user) => user.username), contains('release'));
    expect(find.text('@release'), findsOneWidget);

    await tester.tap(find.text('停用用户'));
    await tester.pumpAndSettle();
    expect(find.text('停用这个用户？'), findsOneWidget);
    await tester.tap(find.text('确认停用'));
    await tester.pumpAndSettle();
    expect(
      data.userItems.firstWhere((user) => user.username == 'release').status,
      'disabled',
    );
  });

  testWidgets('普通用户隐藏系统管理且深链进入用户页显示权限不足', (tester) async {
    final auth = _FeatureAuthGateway(identity: 'user');
    await _pump(
      tester,
      auth,
      FakeMobileDataGateway(profile: fakeIdentity(identity: 'user')),
    );

    await tester.tap(find.text('我的'));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey<String>('user-management-entry')),
      findsNothing,
    );

    final context = tester.element(find.byType(NavigationBar));
    GoRouter.of(context).go('/profile/users');
    await tester.pumpAndSettle();
    expect(find.text('此功能仅管理员可用'), findsOneWidget);
    expect(find.text('部署用户'), findsNothing);
  });

  testWidgets('冷启动管理员深链在会话恢复后返回目标页面', (tester) async {
    final data = FakeMobileDataGateway(
      users: <UserResponse>[
        fakeUser(id: 'user-1', username: 'operator', displayName: '部署用户'),
      ],
    );
    await _pump(
      tester,
      _FeatureAuthGateway(identity: 'administrator'),
      data,
      initialLocation: '/profile/users',
    );
    expect(find.text('用户管理'), findsOneWidget);
    expect(find.text('部署用户'), findsOneWidget);
  });

  testWidgets('认证页 returnTo 不能指向另一个认证页', (tester) async {
    await _pump(
      tester,
      _FeatureAuthGateway(identity: 'administrator'),
      FakeMobileDataGateway(),
      initialLocation: '/login?returnTo=%2Flogin%3Fsource%3Dexternal',
    );

    expect(find.text('可访问应用'), findsOneWidget);
    expect(find.text('登录 Deploy Go'), findsNothing);
  });

  testWidgets('个人资料与偏好保存后更新服务端状态', (tester) async {
    final auth = _FeatureAuthGateway(identity: 'administrator');
    final data = FakeMobileDataGateway();
    await _pump(tester, auth, data);
    await tester.tap(find.text('我的'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('个人资料'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField).first, '值班管理员');
    final saveProfile = find.byKey(
      const ValueKey<String>('save-profile-button'),
    );
    await tester.ensureVisible(saveProfile);
    await tester.pumpAndSettle();
    await tester.tap(saveProfile);
    await tester.pumpAndSettle();
    expect(data.profileValue.displayName, '值班管理员');
    GoRouter.of(tester.element(find.byType(Scaffold).first)).go('/profile');
    await tester.pumpAndSettle();
    expect(find.text('值班管理员'), findsOneWidget);

    GoRouter.of(
      tester.element(find.byType(Scaffold).first),
    ).go('/profile/preferences');
    await tester.pumpAndSettle();
    await tester.tap(find.text('部署完成'));
    final savePreferences = find.byKey(
      const ValueKey<String>('save-preferences-button'),
    );
    await tester.ensureVisible(savePreferences);
    await tester.pumpAndSettle();
    await tester.tap(savePreferences);
    await tester.pumpAndSettle();
    expect(data.preferencesValue.notifyDeploymentCompleted, isTrue);
  });

  testWidgets('退出登录使用确认面板并在清理后返回登录', (tester) async {
    final auth = _FeatureAuthGateway(identity: 'administrator');
    await _pump(tester, auth, FakeMobileDataGateway());
    await tester.tap(find.text('我的'));
    await tester.pumpAndSettle();

    final logout = find.byKey(const ValueKey<String>('logout-button'));
    await tester.drag(find.byType(ListView).last, const Offset(0, -260));
    await tester.pumpAndSettle();
    await tester.tap(logout);
    await tester.pumpAndSettle();
    expect(find.text('退出当前账号？'), findsOneWidget);
    await tester.tap(find.text('确认退出'));
    await tester.pumpAndSettle();
    expect(auth.cleared, isTrue);
    expect(find.text('登录 Deploy Go'), findsOneWidget);
  });

  testWidgets('资源全量失败显示可排查 request ID 和重试入口', (tester) async {
    await _pump(
      tester,
      _FeatureAuthGateway(identity: 'administrator'),
      _FailingMobileDataGateway(),
    );
    expect(find.text('应用列表不可用'), findsOneWidget);
    expect(find.text('Request ID: req-app-list'), findsOneWidget);
    expect(find.text('重试'), findsOneWidget);
  });

  testWidgets('应用和节点空态可切换并保留刷新能力', (tester) async {
    await _pump(
      tester,
      _FeatureAuthGateway(identity: 'administrator'),
      FakeMobileDataGateway(),
    );
    await tester.tap(find.text('资源'));
    await tester.pumpAndSettle();
    expect(find.text('还没有可访问的应用'), findsOneWidget);
    expect(find.byType(RefreshIndicator), findsOneWidget);

    await tester.tap(find.text('节点'));
    await tester.pumpAndSettle();
    expect(find.text('还没有可访问的节点'), findsOneWidget);
  });
}

Future<void> _pump(
  WidgetTester tester,
  _FeatureAuthGateway auth,
  FakeMobileDataGateway data, {
  String? initialLocation,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: <Override>[
        authGatewayProvider.overrideWithValue(auth),
        mobileDataGatewayProvider.overrideWithValue(data),
      ],
      child: DeployGoApp(initialLocation: initialLocation),
    ),
  );
  await tester.pumpAndSettle();
}

class _FeatureAuthGateway implements AuthGateway {
  _FeatureAuthGateway({required String identity})
    : session = SessionResponse(
        (builder) => builder
          ..csrfToken = 'csrf-test'
          ..user.replace(fakeIdentity(identity: identity)),
      );

  SessionResponse? session;
  bool cleared = false;
  final controller = StreamController<void>.broadcast();

  @override
  Stream<void> get unauthorized => controller.stream;

  @override
  Future<void> clearSession() async {
    cleared = true;
    session = null;
  }

  @override
  Future<SessionResponse> login({
    required String username,
    required String password,
  }) async => session!;

  @override
  Future<void> logout() => clearSession();

  @override
  Future<SessionResponse?> restoreSession() async => session;

  @override
  Future<UserIdentity> setup({
    required String username,
    required String password,
    required String displayName,
  }) async => fakeIdentity();

  @override
  Future<SetupStatusResponse> setupStatus() async =>
      SetupStatusResponse((builder) => builder..setupRequired = false);
}

class _FailingMobileDataGateway extends FakeMobileDataGateway {
  @override
  Future<CursorPage<ApplicationResponse>> applications({String? after}) async {
    throw const ApiFailureException(
      ApiFailure(
        status: 503,
        code: 'temporary_failure',
        message: '应用列表不可用',
        requestId: 'req-app-list',
      ),
    );
  }
}
