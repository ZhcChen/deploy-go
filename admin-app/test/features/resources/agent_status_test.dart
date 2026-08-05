import 'dart:async';

import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/api/contracts.dart' as contracts;
import 'package:deploy_go_admin/api/mobile_data_gateway.dart';
import 'package:deploy_go_admin/app/deploy_go_app.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/fake_mobile_data_gateway.dart';

void main() {
  testWidgets('管理员看到在线 Agent 诊断和版本异常但不暴露管理能力', (tester) async {
    final gateway = _CountingGateway(
      nodes: <NodeResponse>[_node(status: 'online')],
      agents: <AgentResponse>[fakeAgent(version: '0.0.9', hostname: 'prod-01')],
    );
    await _pump(tester, gateway, identity: 'administrator');

    expect(find.text('在线'), findsWidgets);
    expect(find.text('0.0.9 · 版本异常'), findsOneWidget);
    expect(find.text('prod-01'), findsOneWidget);
    expect(find.text('x86_64'), findsOneWidget);
    expect(find.textContaining('Web 管理端完成'), findsOneWidget);
    expect(find.textContaining('安装命令'), findsNothing);
    expect(find.textContaining('token'), findsNothing);
  });

  testWidgets('离线 Agent 区分从未连接和曾经在线', (tester) async {
    final neverConnected = _CountingGateway(
      nodes: <NodeResponse>[_node(status: 'offline')],
      agents: <AgentResponse>[
        fakeAgent(
          status: 'offline',
          version: null,
          hostname: null,
          architecture: null,
          lastSeenAt: null,
        ),
      ],
    );
    await _pump(tester, neverConnected, identity: 'administrator');
    expect(find.text('离线'), findsWidgets);
    expect(find.text('从未连接'), findsOneWidget);
    expect(find.text('尚未上报'), findsWidgets);

    await tester.pumpWidget(const SizedBox.shrink());
    final seenBefore = _CountingGateway(
      nodes: <NodeResponse>[_node(status: 'offline')],
      agents: <AgentResponse>[
        fakeAgent(status: 'offline', lastSeenAt: '2026-08-03T02:00:00Z'),
      ],
    );
    await _pump(tester, seenBefore, identity: 'administrator');
    expect(find.text('离线'), findsWidgets);
    expect(find.text('2026-08-03T02:00:00Z'), findsOneWidget);
    expect(find.text('从未连接'), findsNothing);
  });

  testWidgets('普通用户不请求管理员 Agent 接口且前台恢复会刷新状态', (tester) async {
    final ordinaryGateway = _CountingGateway(
      nodes: <NodeResponse>[_node(status: 'offline')],
      agents: <AgentResponse>[fakeAgent(status: 'offline')],
    );
    await _pump(tester, ordinaryGateway, identity: 'user');
    expect(find.text('离线'), findsWidgets);
    expect(ordinaryGateway.agentReads, 0);

    await tester.pumpWidget(const SizedBox.shrink());
    final adminGateway = _CountingGateway(
      nodes: <NodeResponse>[_node(status: 'online')],
      agents: <AgentResponse>[fakeAgent()],
    );
    await _pump(tester, adminGateway, identity: 'administrator');
    expect(adminGateway.agentReads, 1);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(adminGateway.agentReads, 2);
  });

  testWidgets('窄屏和双倍系统字体下诊断内容不溢出', (tester) async {
    tester.view.physicalSize = const Size(320, 720);
    tester.view.devicePixelRatio = 1;
    tester.platformDispatcher.textScaleFactorTestValue = 2;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
      tester.platformDispatcher.clearTextScaleFactorTestValue();
    });
    await _pump(
      tester,
      _CountingGateway(
        nodes: <NodeResponse>[_node(status: 'offline')],
        agents: <AgentResponse>[
          fakeAgent(
            status: 'offline',
            version: '0.0.9-long-build-metadata',
            hostname: 'very-long-production-hostname-01',
          ),
        ],
      ),
      identity: 'administrator',
    );

    expect(tester.takeException(), isNull);
    expect(find.textContaining('版本异常'), findsOneWidget);
  });

  testWidgets('Agent 诊断失败保留节点主体并提供 Request ID 重试', (tester) async {
    await _pump(
      tester,
      _FailingAgentGateway(nodes: <NodeResponse>[_node(status: 'online')]),
      identity: 'administrator',
    );

    expect(find.text('生产节点'), findsOneWidget);
    expect(find.text('Agent 状态暂不可用'), findsOneWidget);
    expect(find.text('Request ID: req-agent-status'), findsOneWidget);
    expect(find.byTooltip('重试 Agent 诊断'), findsOneWidget);
  });
}

Future<void> _pump(
  WidgetTester tester,
  MobileDataGateway gateway, {
  required String identity,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: <Override>[
        authGatewayProvider.overrideWithValue(_AuthGateway(identity)),
        mobileDataGatewayProvider.overrideWithValue(gateway),
      ],
      child: const DeployGoApp(initialLocation: '/resources/nodes/node-1'),
    ),
  );
  await tester.pumpAndSettle();
}

NodeResponse _node({required String status}) => NodeResponse(
  (builder) => builder
    ..id = 'node-1'
    ..name = '生产节点'
    ..status = status
    ..workRoot = '/srv/apps'
    ..checkedAt = '2026-08-03T01:00:00Z'
    ..version = 1
    ..createdAt = '2026-08-02T00:00:00Z'
    ..updatedAt = '2026-08-03T01:00:00Z',
);

class _CountingGateway extends FakeMobileDataGateway {
  _CountingGateway({required super.nodes, required super.agents});

  int agentReads = 0;

  @override
  Future<AgentResponse?> agentForNode(String nodeId) {
    agentReads += 1;
    return super.agentForNode(nodeId);
  }
}

class _FailingAgentGateway extends FakeMobileDataGateway {
  _FailingAgentGateway({required super.nodes});

  @override
  Future<AgentResponse?> agentForNode(String nodeId) async {
    throw const ApiFailureException(
      contracts.ApiFailure(
        status: 503,
        code: 'temporary_failure',
        message: 'Agent 状态暂不可用',
        requestId: 'req-agent-status',
      ),
    );
  }
}

class _AuthGateway implements AuthGateway {
  _AuthGateway(String identity)
    : session = SessionResponse(
        (builder) => builder
          ..csrfToken = 'test-csrf'
          ..user.replace(fakeIdentity(identity: identity)),
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
  }) async => session!.user;

  @override
  Future<SetupStatusResponse> setupStatus() async =>
      SetupStatusResponse((builder) => builder..setupRequired = false);
}
