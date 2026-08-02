import 'dart:async';

import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/api/sse_client.dart';
import 'package:deploy_go_admin/app/deploy_go_app.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import '../support/fake_mobile_data_gateway.dart';

void main() {
  testWidgets('仅选择应用和目标后离开也需确认丢弃', (tester) async {
    final gateway = FakeMobileDataGateway(
      applications: <ApplicationResponse>[fakeApplication()],
      deploymentTargets: <DeploymentTargetResponse>[fakeDeploymentTarget()],
    );
    await _pump(tester, gateway, _TrackingSseClient());
    GoRouter.of(
      tester.element(find.byType(NavigationBar)),
    ).go('/deployments/new');
    await tester.pumpAndSettle();
    await _select(tester, 'deployment-application', '示例应用');
    await _select(
      tester,
      'deployment-target',
      'production · deploy/release.sh',
    );

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    expect(find.text('丢弃未提交的部署配置？'), findsOneWidget);
  });

  testWidgets('preview 后离开需显式丢弃且 confirm 防重复提交', (tester) async {
    final gateway = _DelayedDeploymentGateway();
    final sse = _TrackingSseClient();
    await _pump(tester, gateway, sse);

    final context = tester.element(find.byType(NavigationBar));
    GoRouter.of(context).go('/deployments/new');
    await tester.pumpAndSettle();
    await _select(tester, 'deployment-application', '示例应用');
    await _select(
      tester,
      'deployment-target',
      'production · deploy/release.sh',
    );

    await tester.tap(
      find.byKey(const ValueKey<String>('preview-deployment-button')),
    );
    await tester.pumpAndSettle();
    expect(find.text('Snapshot'), findsOneWidget);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.text('丢弃未提交的部署配置？'), findsOneWidget);
    await tester.tap(find.text('继续编辑'));
    await tester.pumpAndSettle();

    final confirm = find.byKey(
      const ValueKey<String>('confirm-deployment-button'),
    );
    await tester.ensureVisible(confirm);
    await tester.tap(confirm);
    await tester.tap(confirm);
    expect(gateway.confirmCalls, 1);

    gateway.completeConfirm();
    await tester.pumpAndSettle();
    expect(find.text('部署详情'), findsOneWidget);
  });

  testWidgets('进入后台取消 SSE，回到前台先刷新终态', (tester) async {
    final resumeOrder = <String>[];
    final gateway = _OrderedGateway(resumeOrder);
    final sse = _TrackingSseClient();
    await _pump(tester, gateway, sse, auth: _AuthGateway(resumeOrder));
    GoRouter.of(
      tester.element(find.byType(NavigationBar)),
    ).go('/deployments/deployment-1');
    await tester.pumpAndSettle();
    expect(sse.afterValues, <int>[0]);
    expect(find.text('暂停跟随'), findsOneWidget);
    await tester.tap(find.text('暂停跟随'));
    await tester.pumpAndSettle();
    expect(find.text('恢复跟随'), findsOneWidget);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    expect(sse.cancelCount, 1);

    resumeOrder.clear();
    gateway.deploymentItems[0] = gateway.deploymentItems[0].rebuild(
      (builder) => builder
        ..status = 'succeeded'
        ..phase = 'finished'
        ..protocolComplete = true,
    );
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(find.text('成功'), findsOneWidget);
    expect(sse.afterValues, <int>[0]);
    expect(resumeOrder, <String>['session', 'deployment']);
  });

  testWidgets('confirm 失败重试复用同一幂等键', (tester) async {
    final gateway = _RetryConfirmGateway();
    await _pump(tester, gateway, _TrackingSseClient());
    GoRouter.of(
      tester.element(find.byType(NavigationBar)),
    ).go('/deployments/new');
    await tester.pumpAndSettle();
    await _select(tester, 'deployment-application', '示例应用');
    await _select(
      tester,
      'deployment-target',
      'production · deploy/release.sh',
    );
    await tester.tap(
      find.byKey(const ValueKey<String>('preview-deployment-button')),
    );
    await tester.pumpAndSettle();

    final confirm = find.byKey(
      const ValueKey<String>('confirm-deployment-button'),
    );
    await tester.ensureVisible(confirm);
    await tester.tap(confirm);
    await tester.pumpAndSettle();
    expect(find.text('重试'), findsOneWidget);
    await tester.tap(find.text('重试'));
    await tester.pumpAndSettle();

    expect(gateway.keys, hasLength(2));
    expect(gateway.keys.toSet(), hasLength(1));
    expect(find.text('部署详情'), findsOneWidget);
  });

  testWidgets('受控参数在 preview 前校验必填与数值范围', (tester) async {
    final gateway = _SchemaGateway();
    await _pump(tester, gateway, _TrackingSseClient());
    GoRouter.of(
      tester.element(find.byType(NavigationBar)),
    ).go('/deployments/new');
    await tester.pumpAndSettle();
    await _select(tester, 'deployment-application', '示例应用');
    await _select(
      tester,
      'deployment-target',
      'production · deploy/release.sh',
    );

    await tester.tap(
      find.byKey(const ValueKey<String>('preview-deployment-button')),
    );
    await tester.pumpAndSettle();
    expect(find.text('此参数为必填项'), findsOneWidget);
    expect(gateway.previewCalls, 0);

    await tester.enterText(
      find.byKey(const ValueKey<String>('deployment-parameter-replicas')),
      '9',
    );
    await tester.tap(
      find.byKey(const ValueKey<String>('preview-deployment-button')),
    );
    await tester.pumpAndSettle();
    expect(find.text('不能大于 5'), findsOneWidget);
    expect(gateway.previewCalls, 0);
  });
}

Future<void> _select(WidgetTester tester, String key, String option) async {
  await tester.tap(find.byKey(ValueKey<String>(key)));
  await tester.pumpAndSettle();
  await tester.tap(find.text(option).last);
  await tester.pumpAndSettle();
}

Future<void> _pump(
  WidgetTester tester,
  FakeMobileDataGateway gateway,
  DeploymentSseClient sse, {
  AuthGateway? auth,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: <Override>[
        authGatewayProvider.overrideWithValue(auth ?? _AuthGateway()),
        mobileDataGatewayProvider.overrideWithValue(gateway),
        deploymentSseClientProvider.overrideWithValue(sse),
      ],
      child: const DeployGoApp(),
    ),
  );
  await tester.pumpAndSettle();
}

class _DelayedDeploymentGateway extends FakeMobileDataGateway {
  _DelayedDeploymentGateway()
    : super(
        applications: <ApplicationResponse>[fakeApplication()],
        deploymentTargets: <DeploymentTargetResponse>[fakeDeploymentTarget()],
      );

  final completer = Completer<DeploymentResponse>();
  int confirmCalls = 0;

  @override
  Future<DeploymentResponse> confirmDeployment({
    required DeploymentPreviewResponse preview,
    required Map<String, Object?> parameters,
    required String idempotencyKey,
  }) {
    confirmCalls += 1;
    return completer.future;
  }

  void completeConfirm() {
    final deployment = fakeDeployment(id: 'deployment-1');
    deploymentItems.add(deployment);
    completer.complete(deployment);
  }
}

class _TrackingSseClient implements DeploymentSseClient {
  final afterValues = <int>[];
  int cancelCount = 0;

  @override
  Stream<SseEvent> deploymentLogs(String deploymentId, {int after = 0}) {
    afterValues.add(after);
    late final StreamController<SseEvent> controller;
    controller = StreamController<SseEvent>(
      onCancel: () {
        cancelCount += 1;
        return controller.close();
      },
    );
    return controller.stream;
  }
}

class _OrderedGateway extends FakeMobileDataGateway {
  _OrderedGateway(this.order)
    : super(
        deployments: <DeploymentResponse>[fakeDeployment(id: 'deployment-1')],
      );

  final List<String> order;

  @override
  Future<DeploymentResponse> deployment(String id) {
    order.add('deployment');
    return super.deployment(id);
  }
}

class _RetryConfirmGateway extends FakeMobileDataGateway {
  _RetryConfirmGateway()
    : super(
        applications: <ApplicationResponse>[fakeApplication()],
        deploymentTargets: <DeploymentTargetResponse>[fakeDeploymentTarget()],
      );

  final keys = <String>[];

  @override
  Future<DeploymentResponse> confirmDeployment({
    required DeploymentPreviewResponse preview,
    required Map<String, Object?> parameters,
    required String idempotencyKey,
  }) async {
    keys.add(idempotencyKey);
    if (keys.length == 1) throw StateError('temporary failure');
    final deployment = fakeDeployment(id: 'deployment-retried');
    deploymentItems.add(deployment);
    return deployment;
  }
}

class _SchemaGateway extends FakeMobileDataGateway {
  _SchemaGateway()
    : super(
        applications: <ApplicationResponse>[fakeApplication()],
        deploymentTargets: <DeploymentTargetResponse>[
          fakeDeploymentTarget().rebuild(
            (builder) => builder.parameterSchema = JsonObject(<String, Object>{
              'type': 'object',
              'required': <String>['replicas'],
              'properties': <String, Object>{
                'replicas': <String, Object>{
                  'type': 'integer',
                  'title': '副本数',
                  'minimum': 1,
                  'maximum': 5,
                },
              },
            }),
          ),
        ],
      );

  int previewCalls = 0;

  @override
  Future<DeploymentPreviewResponse> previewDeployment(
    String targetId,
    Map<String, Object?> parameters,
  ) {
    previewCalls += 1;
    return super.previewDeployment(targetId, parameters);
  }
}

class _AuthGateway implements AuthGateway {
  _AuthGateway([this.order]);

  final List<String>? order;
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
  Future<SessionResponse?> restoreSession() async {
    order?.add('session');
    return session;
  }

  @override
  Future<UserIdentity> setup({
    required String setupToken,
    required String username,
    required String password,
    required String displayName,
  }) async => session.user;

  @override
  Future<SetupStatusResponse> setupStatus() async => SetupStatusResponse(
    (builder) => builder
      ..setupRequired = false
      ..setupEnabled = false,
  );
}
