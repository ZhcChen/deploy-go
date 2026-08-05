import 'dart:async';

import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/app/deploy_go_app.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_admin/features/resources/resources_pages.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/fake_mobile_data_gateway.dart';

void main() {
  test('远端登出失败后仍清理本地会话并进入未登录态', () async {
    final gateway = _FakeAuthGateway(session: _session())..logoutFails = true;
    final container = ProviderContainer(
      overrides: <Override>[authGatewayProvider.overrideWithValue(gateway)],
    );
    addTearDown(container.dispose);
    final controller = container.read(sessionControllerProvider.notifier);
    await controller.bootstrap();

    await controller.logout();

    expect(
      container.read(sessionControllerProvider).phase,
      SessionPhase.unauthenticated,
    );
    expect(gateway.clearCalls, 1);
    expect(gateway.session, isNull);
  });

  testWidgets('恢复会话后展示四项导航且激活态清晰', (tester) async {
    final gateway = _FakeAuthGateway(session: _session());
    await tester.pumpWidget(
      ProviderScope(
        overrides: <Override>[
          authGatewayProvider.overrideWithValue(gateway),
          mobileDataGatewayProvider.overrideWithValue(FakeMobileDataGateway()),
        ],
        child: const DeployGoApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('概览'), findsWidgets);
    expect(find.text('资源'), findsOneWidget);
    expect(find.text('部署'), findsOneWidget);
    expect(find.text('我的'), findsOneWidget);

    final navigation = tester.widget<NavigationBar>(find.byType(NavigationBar));
    expect(navigation.selectedIndex, 0);
    expect(find.bySemanticsLabel('可访问应用 0'), findsOneWidget);
    await tester.tap(find.text('部署'));
    await tester.pumpAndSettle();
    expect(
      tester.widget<NavigationBar>(find.byType(NavigationBar)).selectedIndex,
      2,
    );
    expect(
      find.byKey(const ValueKey<String>('deployment-root')),
      findsOneWidget,
    );

    gateway.unauthorizedController.add(null);
    await tester.pumpAndSettle();
    expect(find.text('登录 Deploy Go'), findsOneWidget);
  });

  testWidgets('窄屏和 200% 字体下主要页面不溢出且触控目标不少于 44', (tester) async {
    tester.view.physicalSize = const Size(640, 1136);
    tester.view.devicePixelRatio = 2;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(textScaler: TextScaler.linear(2)),
        child: ProviderScope(
          overrides: <Override>[
            authGatewayProvider.overrideWithValue(
              _FakeAuthGateway(session: _session()),
            ),
            mobileDataGatewayProvider.overrideWithValue(
              FakeMobileDataGateway(),
            ),
          ],
          child: const DeployGoApp(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('资源'));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byType(NavigationBar)).height,
      greaterThanOrEqualTo(64),
    );
    expect(find.byType(SegmentedButton<ResourceSegment>), findsOneWidget);
  });

  testWidgets('需要初始化时 setup 完成后进入登录且不保留输入', (tester) async {
    final gateway = _FakeAuthGateway(setupRequired: true);
    await tester.pumpWidget(
      ProviderScope(
        overrides: <Override>[
          authGatewayProvider.overrideWithValue(gateway),
          mobileDataGatewayProvider.overrideWithValue(FakeMobileDataGateway()),
        ],
        child: const DeployGoApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('初始化管理员'), findsOneWidget);
    await tester.enterText(
      find.byKey(const ValueKey<String>('setup-username')),
      'admin',
    );
    await tester.enterText(
      find.byKey(const ValueKey<String>('setup-display-name')),
      '管理员',
    );
    await tester.enterText(
      find.byKey(const ValueKey<String>('setup-password')),
      'initial-password',
    );
    await tester.tap(find.widgetWithText(FilledButton, '完成初始化'));
    await tester.pumpAndSettle();

    expect(find.text('登录 Deploy Go'), findsOneWidget);
    expect(find.text('initial-password'), findsNothing);
  });

  testWidgets('401 等待安全会话清理完成后再返回登录', (tester) async {
    final gateway = _FakeAuthGateway(session: _session());
    await tester.pumpWidget(
      ProviderScope(
        overrides: <Override>[
          authGatewayProvider.overrideWithValue(gateway),
          mobileDataGatewayProvider.overrideWithValue(FakeMobileDataGateway()),
        ],
        child: const DeployGoApp(),
      ),
    );
    await tester.pumpAndSettle();

    gateway.clearCompleter = Completer<void>();
    gateway.unauthorizedController.add(null);
    await tester.pump();
    expect(find.text('登录 Deploy Go'), findsNothing);

    gateway.clearCompleter!.complete();
    await tester.pumpAndSettle();
    expect(find.text('登录 Deploy Go'), findsOneWidget);
  });
}

class _FakeAuthGateway implements AuthGateway {
  _FakeAuthGateway({this.setupRequired = false, this.session});

  bool setupRequired;
  SessionResponse? session;
  Completer<void>? clearCompleter;
  bool logoutFails = false;
  int clearCalls = 0;
  final unauthorizedController = StreamController<void>.broadcast();

  @override
  Stream<void> get unauthorized => unauthorizedController.stream;

  @override
  Future<void> clearSession() async {
    clearCalls += 1;
    await clearCompleter?.future;
    session = null;
  }

  @override
  Future<SessionResponse> login({
    required String username,
    required String password,
  }) async => session = _session();

  @override
  Future<void> logout() async {
    if (logoutFails) throw StateError('remote logout failed');
    await clearSession();
  }

  @override
  Future<SessionResponse?> restoreSession() async => session;

  @override
  Future<UserIdentity> setup({
    required String username,
    required String password,
    required String displayName,
  }) async {
    setupRequired = false;
    return _user();
  }

  @override
  Future<SetupStatusResponse> setupStatus() async =>
      SetupStatusResponse((builder) => builder..setupRequired = setupRequired);
}

UserIdentity _user() => UserIdentity(
  (builder) => builder
    ..id = 'admin-1'
    ..username = 'admin'
    ..displayName = '管理员'
    ..identity = 'administrator',
);

SessionResponse _session() => SessionResponse(
  (builder) => builder
    ..csrfToken = 'test-csrf'
    ..user.replace(_user()),
);
