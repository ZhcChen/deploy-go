import 'dart:async';

import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/app/deploy_go_app.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import '../test/support/fake_mobile_data_gateway.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('管理员恢复会话后可从我的进入用户管理', (tester) async {
    final auth = _IntegrationAuthGateway();
    final data = FakeMobileDataGateway(
      users: <UserResponse>[
        fakeUser(id: 'user-1', username: 'operator', displayName: '部署用户'),
      ],
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: <Override>[
          authGatewayProvider.overrideWithValue(auth),
          mobileDataGatewayProvider.overrideWithValue(data),
        ],
        child: const DeployGoApp(),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('我的'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('用户管理'));
    await tester.pumpAndSettle();

    expect(find.text('部署用户'), findsOneWidget);
    expect(find.textContaining('角色'), findsNothing);
    expect(find.textContaining('邀请'), findsNothing);
  });

  testWidgets('节点详情只读展示 Agent 状态且不暴露安装命令', (tester) async {
    final data = FakeMobileDataGateway(
      nodes: <NodeResponse>[
        NodeResponse(
          (builder) => builder
            ..id = 'node-1'
            ..name = '生产节点'
            ..status = 'online'
            ..workRoot = '/srv/apps'
            ..version = 1
            ..createdAt = '2026-08-02T00:00:00Z'
            ..updatedAt = '2026-08-03T00:00:00Z',
        ),
      ],
      agents: <AgentResponse>[fakeAgent()],
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: <Override>[
          authGatewayProvider.overrideWithValue(_IntegrationAuthGateway()),
          mobileDataGatewayProvider.overrideWithValue(data),
        ],
        child: const DeployGoApp(initialLocation: '/resources/nodes/node-1'),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('在线'), findsWidgets);
    expect(find.text('0.1.0'), findsOneWidget);
    expect(find.textContaining('安装命令'), findsNothing);
    expect(find.textContaining('token'), findsNothing);
  });
}

class _IntegrationAuthGateway implements AuthGateway {
  _IntegrationAuthGateway()
    : session = SessionResponse(
        (builder) => builder
          ..csrfToken = 'test-csrf'
          ..user.replace(fakeIdentity()),
      );

  SessionResponse? session;
  final controller = StreamController<void>.broadcast();

  @override
  Stream<void> get unauthorized => controller.stream;

  @override
  Future<void> clearSession() async => session = null;

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
