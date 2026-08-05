import 'dart:async';

import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/api/sse_client.dart';
import 'package:deploy_go_admin/app/deploy_go_app.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_admin/features/deployments/deployment_pages.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:integration_test/integration_test.dart';

import '../test/support/fake_mobile_data_gateway.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('部署详情在后台释放日志连接并按终态恢复', (tester) async {
    final data = FakeMobileDataGateway(
      deployments: <DeploymentResponse>[
        fakeDeployment(id: 'deployment-lifecycle'),
      ],
    );
    final sse = _LifecycleSseClient();
    await tester.pumpWidget(
      ProviderScope(
        overrides: <Override>[
          authGatewayProvider.overrideWithValue(_LifecycleAuthGateway()),
          mobileDataGatewayProvider.overrideWithValue(data),
          deploymentSseClientProvider.overrideWithValue(sse),
        ],
        child: const DeployGoApp(),
      ),
    );
    await tester.pumpAndSettle();
    GoRouter.of(
      tester.element(find.byType(NavigationBar)),
    ).go('/deployments/deployment-lifecycle');
    await tester.pumpAndSettle();
    expect(sse.connections, 1);

    final dynamic pageState = tester.state(find.byType(DeploymentDetailPage));
    pageState.didChangeAppLifecycleState(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    expect(sse.cancellations, 1);

    data.deploymentItems[0] = data.deploymentItems[0].rebuild(
      (builder) => builder
        ..status = 'succeeded'
        ..phase = 'finished'
        ..protocolComplete = true,
    );
    pageState.didChangeAppLifecycleState(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(find.text('成功'), findsOneWidget);
    expect(sse.connections, 1);
  });
}

class _LifecycleSseClient implements DeploymentSseClient {
  int connections = 0;
  int cancellations = 0;

  @override
  Stream<SseEvent> deploymentLogs(String deploymentId, {int after = 0}) {
    connections += 1;
    late final StreamController<SseEvent> controller;
    controller = StreamController<SseEvent>(
      onCancel: () {
        cancellations += 1;
        return controller.close();
      },
    );
    return controller.stream;
  }
}

class _LifecycleAuthGateway implements AuthGateway {
  final session = SessionResponse(
    (builder) => builder
      ..csrfToken = 'test-csrf'
      ..user.replace(fakeIdentity()),
  );

  @override
  Stream<void> get unauthorized => const Stream<void>.empty();
  @override
  Future<void> clearSession() async {}
  @override
  Future<SessionResponse> login({
    required String username,
    required String password,
  }) async => session;
  @override
  Future<void> logout() async {}
  @override
  Future<SessionResponse?> restoreSession() async => session;
  @override
  Future<UserIdentity> setup({
    required String username,
    required String password,
    required String displayName,
  }) async => session.user;
  @override
  Future<SetupStatusResponse> setupStatus() async =>
      SetupStatusResponse((builder) => builder..setupRequired = false);
}
