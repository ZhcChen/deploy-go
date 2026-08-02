import 'dart:async';
import 'dart:convert';

import 'package:deploy_go_admin/api/sse_client.dart';
import 'package:deploy_go_admin/api/contracts.dart';
import 'package:deploy_go_admin/api/mobile_data_gateway.dart';
import 'package:deploy_go_admin/app/providers.dart';
import 'package:deploy_go_admin/features/deployments/deployment_providers.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../support/fake_mobile_data_gateway.dart';

void main() {
  test('部署列表页离开后销毁账号级缓存', () async {
    final container = ProviderContainer(
      overrides: <Override>[
        mobileDataGatewayProvider.overrideWithValue(
          FakeMobileDataGateway(
            deployments: <DeploymentResponse>[
              fakeDeployment(id: 'account-a-deployment'),
            ],
          ),
        ),
      ],
    );
    addTearDown(container.dispose);
    final subscription = container.listen(
      deploymentsProvider,
      (_, _) {},
      fireImmediately: true,
    );
    await pumpEventQueue();
    expect(
      container.read(deploymentsProvider).items.single.id,
      'account-a-deployment',
    );

    subscription.close();
    await pumpEventQueue();

    expect(container.exists(deploymentsProvider), isFalse);
  });

  test('重复日志去重，后台释放连接，前台先刷新终态', () async {
    final gateway = _DeploymentGateway();
    final sse = _FakeSseClient();
    final controller = DeploymentDetailController('deployment-1', gateway, sse);
    addTearDown(controller.dispose);

    await controller.initialize();
    expect(gateway.detailCalls, 1);
    expect(sse.afterValues, <int>[0]);

    sse.add(_logEvent(1, 'first'));
    sse.add(_logEvent(1, 'duplicate'));
    await _waitForLogFlush(controller);
    expect(controller.state.logs.single.content, 'first');
    expect(controller.state.lastEventId, 1);

    await controller.enterBackground();
    expect(sse.cancelCount, 1);
    gateway.current = gateway.current.rebuild(
      (builder) => builder
        ..status = 'succeeded'
        ..phase = 'finished'
        ..protocolComplete = true,
    );
    await controller.enterForeground();

    expect(gateway.detailCalls, 2);
    expect(controller.state.deployment?.status, 'succeeded');
    expect(sse.afterValues, <int>[0]);
    expect(controller.state.connection, SseConnectionState.ended);
  });

  test('授权撤销立即清理部署和日志缓存', () async {
    final gateway = _DeploymentGateway();
    final sse = _FakeSseClient();
    final controller = DeploymentDetailController('deployment-1', gateway, sse);
    addTearDown(controller.dispose);
    await controller.initialize();
    sse.add(_logEvent(1, 'visible-before-revoke'));
    await _waitForLogFlush(controller);

    sse.add(
      SseEvent(
        id: '2',
        event: 'authorization-revoked',
        data: jsonEncode(<String, String>{
          'code': 'forbidden',
          'message': '授权已失效',
          'request_id': 'req-sse-forbidden',
        }),
      ),
    );
    await pumpEventQueue();

    expect(controller.state.deployment, isNull);
    expect(controller.state.logs, isEmpty);
    expect(controller.state.lastEventId, 0);
    expect(
      (controller.state.error as ApiFailureException).failure.requestId,
      'req-sse-forbidden',
    );
  });

  test('cancel 直接返回终态时连接状态立即结束', () async {
    final gateway = _DeploymentGateway();
    final sse = _FakeSseClient();
    final controller = DeploymentDetailController('deployment-1', gateway, sse);
    addTearDown(controller.dispose);
    await controller.initialize();
    gateway.current = gateway.current.rebuild(
      (builder) => builder
        ..status = 'canceled'
        ..phase = 'canceled',
    );

    await controller.cancel();

    expect(controller.state.deployment?.status, 'canceled');
    expect(controller.state.connection, SseConnectionState.ended);
    expect(sse.cancelCount, 1);
  });

  test('cancel 和 retry 失败后操作锁回滚且可重试', () async {
    final gateway = _DeploymentGateway()
      ..cancelError = StateError('cancel failed')
      ..retryError = StateError('retry failed');
    final controller = DeploymentDetailController(
      'deployment-1',
      gateway,
      _FakeSseClient(),
    );
    addTearDown(controller.dispose);
    await controller.initialize();

    await controller.cancel();
    expect(controller.state.action, isNull);
    expect(controller.state.actionError, isA<StateError>());

    final retried = await controller.retry('retry-key');
    expect(retried, isNull);
    expect(controller.state.action, isNull);
    expect(gateway.retryCalls, 1);
  });

  test('写操作返回 403 时清理已缓存的部署和日志', () async {
    final gateway = _DeploymentGateway()
      ..cancelError = const ApiFailureException(
        ApiFailure(
          status: 403,
          code: 'forbidden',
          message: '禁止访问',
          requestId: 'req-forbidden',
        ),
      );
    final sse = _FakeSseClient();
    final controller = DeploymentDetailController('deployment-1', gateway, sse);
    addTearDown(controller.dispose);
    await controller.initialize();
    sse.add(_logEvent(1, 'must-be-cleared'));
    await _waitForLogFlush(controller);

    await controller.cancel();

    expect(controller.state.deployment, isNull);
    expect(controller.state.logs, isEmpty);
    expect(controller.state.error, isA<ApiFailureException>());
  });

  test('detail 和 retry 返回 403 时不保留受保护内容', () async {
    const forbidden = ApiFailureException(
      ApiFailure(
        status: 403,
        code: 'forbidden',
        message: '禁止访问',
        requestId: 'req-forbidden-matrix',
      ),
    );
    final detailGateway = _DeploymentGateway()..detailError = forbidden;
    final detailSse = _FakeSseClient();
    final detailController = DeploymentDetailController(
      'deployment-1',
      detailGateway,
      detailSse,
    );
    addTearDown(detailController.dispose);

    await detailController.initialize();

    expect(detailController.state.deployment, isNull);
    expect(detailController.state.logs, isEmpty);
    expect(detailSse.afterValues, isEmpty);

    final retryGateway = _DeploymentGateway()
      ..current = fakeDeployment(id: 'deployment-1', status: 'failed')
      ..retryError = forbidden;
    final retryController = DeploymentDetailController(
      'deployment-1',
      retryGateway,
      _FakeSseClient(),
    );
    addTearDown(retryController.dispose);
    await retryController.initialize();

    await retryController.retry('retry-forbidden-key');

    expect(retryController.state.deployment, isNull);
    expect(retryController.state.logs, isEmpty);
    expect(
      (retryController.state.error as ApiFailureException).failure.requestId,
      'req-forbidden-matrix',
    );
  });

  test('日志文本过滤控制字符和方向控制符', () {
    expect(sanitizeLogText('ok\u0001\u202esecret'), 'ok��secret');
  });

  test('大批量日志保持 1000 条窗口且写操作仍可执行', () async {
    final gateway = _DeploymentGateway();
    final sse = _FakeSseClient();
    final controller = DeploymentDetailController('deployment-1', gateway, sse);
    addTearDown(controller.dispose);
    await controller.initialize();
    for (var sequence = 1; sequence <= 1100; sequence += 1) {
      sse.add(_logEvent(sequence, 'line-$sequence'));
    }
    await _waitForLogFlush(controller, expectedCount: 1000);

    expect(controller.state.logs, hasLength(1000));
    expect(controller.state.logs.first.sequence, 101);
    expect(controller.state.logs.last.sequence, 1100);

    await controller.cancel();

    expect(controller.state.action, isNull);
    expect(gateway.cancelCalls, 1);
  });
}

Future<void> _waitForLogFlush(
  DeploymentDetailController controller, {
  int expectedCount = 1,
}) async {
  final deadline = DateTime.now().add(const Duration(seconds: 1));
  while (controller.state.logs.length < expectedCount &&
      DateTime.now().isBefore(deadline)) {
    await Future<void>.delayed(const Duration(milliseconds: 5));
  }
}

SseEvent _logEvent(int sequence, String content) => SseEvent(
  id: '$sequence',
  event: 'log',
  data: jsonEncode(<String, Object>{
    'sequence': sequence,
    'stream': 'stdout',
    'content': content,
    'truncated': false,
    'created_at': '2026-08-02T00:00:00Z',
  }),
);

class _DeploymentGateway extends FakeMobileDataGateway {
  _DeploymentGateway()
    : current = fakeDeployment(id: 'deployment-1'),
      super(deployments: <DeploymentResponse>[]);

  DeploymentResponse current;
  int detailCalls = 0;
  int retryCalls = 0;
  int cancelCalls = 0;
  Object? cancelError;
  Object? retryError;
  Object? detailError;

  @override
  Future<DeploymentResponse> deployment(String id) async {
    detailCalls += 1;
    if (detailError != null) throw detailError!;
    return current;
  }

  @override
  Future<DeploymentResponse> cancelDeployment(String id) async {
    cancelCalls += 1;
    if (cancelError != null) throw cancelError!;
    return current;
  }

  @override
  Future<DeploymentResponse> retryDeployment(
    String id,
    String idempotencyKey,
  ) async {
    retryCalls += 1;
    if (retryError != null) throw retryError!;
    return current;
  }
}

class _FakeSseClient implements DeploymentSseClient {
  final controllers = <StreamController<SseEvent>>[];
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
    controllers.add(controller);
    return controller.stream;
  }

  void add(SseEvent event) => controllers.last.add(event);
}
