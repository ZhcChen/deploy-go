import 'dart:convert';

import 'package:deploy_go_admin/api/api_environment.dart';
import 'package:deploy_go_admin/api/auth_repository.dart';
import 'package:deploy_go_admin/api/deploy_go_api.dart';
import 'package:deploy_go_admin/api/mobile_data_gateway.dart';
import 'package:deploy_go_admin/security/secure_session_store.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/memory_secure_store.dart';

void main() {
  test('登录和会话恢复发送精确 Origin、Fetch Metadata 并恢复 Cookie', () async {
    final requests = <RequestOptions>[];
    final logs = <String>[];
    final adapter = _RecordingAdapter(requests);
    final secureStore = MemorySecureStore();
    const environment = ApiEnvironment(
      baseUrl: 'https://api.example.test',
      allowedOrigin: 'https://app.example.test',
    );

    final first = await DeployGoApi.create(
      environment: environment,
      secureStore: secureStore,
      adapter: adapter,
      logSink: logs.add,
    );
    final repository = AuthRepository(first, SecureSessionStore(secureStore));
    await repository.login(username: 'operator', password: 'secret-value');

    final login = requests.single;
    expect(login.path, '/api/v1/auth/login');
    expect(login.headers['Origin'], environment.allowedOrigin);
    expect(login.headers['Cookie'], isNull);
    expect(jsonEncode(login.data), isNot(contains('csrf-token')));

    requests.clear();
    final restored = await DeployGoApi.create(
      environment: environment,
      secureStore: secureStore,
      adapter: adapter,
    );
    final session = await AuthRepository(
      restored,
      SecureSessionStore(secureStore),
    ).restoreSession();

    expect(session, isNotNull);
    expect(requests.map((request) => request.path), <String>[
      '/api/v1/auth/me',
      '/api/v1/auth/csrf',
    ]);
    expect(requests.first.headers['Cookie'], 'deploy_go_session=session-value');
    expect(
      requests.last.headers,
      containsPair('Origin', environment.allowedOrigin),
    );
    expect(
      requests.last.headers,
      containsPair('Sec-Fetch-Site', 'same-origin'),
    );
    expect(requests.last.headers, containsPair('Sec-Fetch-Mode', 'cors'));
    expect(
      await SecureSessionStore(secureStore).readCsrfToken(),
      'refreshed-csrf',
    );
    expect(logs.join('\n'), contains('/api/v1/auth/login'));
    expect(logs.join('\n'), isNot(contains('secret-value')));
    expect(logs.join('\n'), isNot(contains('session-value')));
    expect(logs.join('\n'), isNot(contains('refreshed-csrf')));
  });

  test('受保护请求返回 401 时发布会话失效事件', () async {
    final api = await DeployGoApi.create(
      environment: const ApiEnvironment(
        baseUrl: 'https://api.example.test',
        allowedOrigin: 'https://app.example.test',
      ),
      secureStore: MemorySecureStore(),
      adapter: _UnauthorizedAdapter(),
    );
    var unauthorized = 0;
    final subscription = api.unauthorized.listen((_) => unauthorized += 1);

    await expectLater(
      api.client.getAuthApi().authMe(),
      throwsA(isA<DioException>()),
    );
    await Future<void>.delayed(Duration.zero);
    expect(unauthorized, 1);
    await subscription.cancel();
  });

  test('移动业务网关从安全存储注入 CSRF 并保留错误 request ID', () async {
    final requests = <RequestOptions>[];
    final secureStore = MemorySecureStore();
    await SecureSessionStore(secureStore).writeCsrfToken('mobile-csrf');
    final api = await DeployGoApi.create(
      environment: const ApiEnvironment(
        baseUrl: 'https://api.example.test',
        allowedOrigin: 'https://app.example.test',
      ),
      secureStore: secureStore,
      adapter: _MobileGatewayAdapter(requests),
    );
    final gateway = DeployGoMobileDataGateway(
      api,
      SecureSessionStore(secureStore),
    );

    final profile = await gateway.updateProfile('值班管理员');
    expect(profile.displayName, '值班管理员');
    expect(requests.single.headers['X-CSRF-Token'], 'mobile-csrf');
    expect(jsonEncode(requests.single.data), contains('值班管理员'));

    await expectLater(
      gateway.users(),
      throwsA(
        isA<ApiFailureException>()
            .having((error) => error.failure.status, 'status', 403)
            .having(
              (error) => error.failure.requestId,
              'request ID',
              'req-mobile-forbidden',
            ),
      ),
    );
  });

  test('部署 preview 与 confirm 注入 CSRF 并传递幂等键', () async {
    final requests = <RequestOptions>[];
    final secureStore = MemorySecureStore();
    await SecureSessionStore(secureStore).writeCsrfToken('fixture-csrf');
    final api = await DeployGoApi.create(
      environment: const ApiEnvironment(
        baseUrl: 'https://api.example.test',
        allowedOrigin: 'https://app.example.test',
      ),
      secureStore: secureStore,
      adapter: _DeploymentGatewayAdapter(requests),
    );
    final gateway = DeployGoMobileDataGateway(
      api,
      SecureSessionStore(secureStore),
    );

    final preview = await gateway.previewDeployment(
      'target-1',
      <String, Object?>{'release': '2026.08.02'},
    );
    final deployment = await gateway.confirmDeployment(
      preview: preview,
      parameters: const <String, Object?>{'release': '2026.08.02'},
      idempotencyKey: 'deploy-fixture-key',
    );

    expect(deployment.id, 'deployment-1');
    expect(requests, hasLength(2));
    expect(
      requests.map((request) => request.headers['X-CSRF-Token']).toSet(),
      <Object?>{'fixture-csrf'},
    );
    expect(requests.last.headers['Idempotency-Key'], 'deploy-fixture-key');
    expect(jsonEncode(requests.last.data), contains(preview.snapshotHash));
  });
}

class _RecordingAdapter implements HttpClientAdapter {
  _RecordingAdapter(this.requests);

  final List<RequestOptions> requests;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<List<int>>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(options);
    final (status, body, headers) = switch (options.path) {
      '/api/v1/auth/login' => (
        200,
        _session('login-csrf'),
        <String, List<String>>{
          Headers.contentTypeHeader: <String>['application/json'],
          'set-cookie': <String>[
            'deploy_go_session=session-value; Path=/; HttpOnly; Secure; SameSite=Lax',
          ],
        },
      ),
      '/api/v1/auth/me' => (
        200,
        _user,
        <String, List<String>>{
          Headers.contentTypeHeader: <String>['application/json'],
        },
      ),
      '/api/v1/auth/csrf' => (
        200,
        '{"csrf_token":"refreshed-csrf"}',
        <String, List<String>>{
          Headers.contentTypeHeader: <String>['application/json'],
        },
      ),
      _ => (404, '{}', <String, List<String>>{}),
    };
    return ResponseBody.fromString(body, status, headers: headers);
  }

  @override
  void close({bool force = false}) {}
}

class _UnauthorizedAdapter implements HttpClientAdapter {
  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<List<int>>? requestStream,
    Future<void>? cancelFuture,
  ) async => ResponseBody.fromString(
    '{"code":"not_authenticated","message":"未登录","request_id":"req-test"}',
    401,
    headers: <String, List<String>>{
      Headers.contentTypeHeader: <String>['application/json'],
    },
  );

  @override
  void close({bool force = false}) {}
}

class _MobileGatewayAdapter implements HttpClientAdapter {
  _MobileGatewayAdapter(this.requests);
  final List<RequestOptions> requests;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<List<int>>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    if (options.path == '/api/v1/auth/profile') {
      requests.add(options);
      return ResponseBody.fromString(
        '{"id":"admin-1","username":"admin","display_name":"值班管理员","email":null,"identity":"administrator"}',
        200,
        headers: <String, List<String>>{
          Headers.contentTypeHeader: <String>['application/json'],
        },
      );
    }
    return ResponseBody.fromString(
      '{"code":"forbidden","message":"权限不足","request_id":"req-mobile-forbidden"}',
      403,
      headers: <String, List<String>>{
        Headers.contentTypeHeader: <String>['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

class _DeploymentGatewayAdapter implements HttpClientAdapter {
  _DeploymentGatewayAdapter(this.requests);
  final List<RequestOptions> requests;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<List<int>>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(options);
    final body = options.path.endsWith('/deployment-preview')
        ? '{"application_id":"app-1","application_name":"示例应用","environment":"production","execution_mode":"script","node_id":"node-1","node_name":"示例节点","parameters":{"release":"2026.08.02"},"script_path":"deploy/release.sh","snapshot_hash":"snapshot-1","target_id":"target-1"}'
        : '{"id":"deployment-1","target_id":"target-1","requested_by":"admin-1","status":"queued","execution_mode":"script","phase":"queued","snapshot_hash":"snapshot-1","stage_tasks":[],"protocol_complete":false,"version":1,"created_at":"2026-08-02T00:00:00Z","updated_at":"2026-08-02T00:00:00Z","queued_at":"2026-08-02T00:00:00Z"}';
    return ResponseBody.fromString(
      body,
      200,
      headers: <String, List<String>>{
        Headers.contentTypeHeader: <String>['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

const _user =
    '{"id":"user-1","username":"operator","display_name":"部署用户","email":null,"identity":"user"}';
String _session(String csrf) => '{"csrf_token":"$csrf","user":$_user}';
