import 'dart:async';

import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/auth_repository.dart';
import '../api/mobile_data_gateway.dart';
import '../api/sse_client.dart';

final authGatewayProvider = Provider<AuthGateway>(
  (ref) => throw StateError('AuthGateway 尚未初始化'),
);

final mobileDataGatewayProvider = Provider<MobileDataGateway>(
  (ref) => throw StateError('MobileDataGateway 尚未初始化'),
);

final deploymentSseClientProvider = Provider<DeploymentSseClient>(
  (ref) => throw StateError('DeploymentSseClient 尚未初始化'),
);

final sessionControllerProvider =
    StateNotifierProvider<SessionController, SessionState>((ref) {
      final controller = SessionController(ref.watch(authGatewayProvider));
      controller.bootstrap();
      return controller;
    });

enum SessionPhase {
  bootstrapping,
  setupRequired,
  unauthenticated,
  authenticated,
  failure,
}

class SessionState {
  const SessionState(this.phase, {this.session, this.message});

  const SessionState.bootstrapping() : this(SessionPhase.bootstrapping);

  final SessionPhase phase;
  final SessionResponse? session;
  final String? message;
}

class SessionController extends StateNotifier<SessionState> {
  SessionController(this._auth) : super(const SessionState.bootstrapping()) {
    _unauthorizedSubscription = _auth.unauthorized.listen((_) {
      unawaited(_clearUnauthorizedSession());
    });
  }

  final AuthGateway _auth;
  late final StreamSubscription<void> _unauthorizedSubscription;
  SessionState get current => state;

  Future<void> _clearUnauthorizedSession() async {
    try {
      await _auth.clearSession();
    } finally {
      if (mounted) {
        state = const SessionState(SessionPhase.unauthenticated);
      }
    }
  }

  @override
  void dispose() {
    _unauthorizedSubscription.cancel();
    super.dispose();
  }

  Future<void> bootstrap() async {
    state = const SessionState.bootstrapping();
    try {
      final setup = await _auth.setupStatus();
      if (setup.setupRequired) {
        state = const SessionState(SessionPhase.setupRequired);
        return;
      }
      final session = await _auth.restoreSession();
      state = session == null
          ? const SessionState(SessionPhase.unauthenticated)
          : SessionState(SessionPhase.authenticated, session: session);
    } catch (_) {
      state = const SessionState(SessionPhase.failure, message: '无法连接部署控制服务');
    }
  }

  Future<void> login(String username, String password) async {
    state = const SessionState.bootstrapping();
    try {
      final session = await _auth.login(username: username, password: password);
      state = SessionState(SessionPhase.authenticated, session: session);
    } catch (_) {
      state = const SessionState(
        SessionPhase.unauthenticated,
        message: '用户名或密码无效',
      );
    }
  }

  Future<void> setup({
    required String username,
    required String password,
    required String displayName,
  }) async {
    state = const SessionState.bootstrapping();
    try {
      await _auth.setup(
        username: username,
        password: password,
        displayName: displayName,
      );
      state = const SessionState(SessionPhase.unauthenticated);
    } catch (_) {
      state = const SessionState(
        SessionPhase.setupRequired,
        message: '初始化失败，请检查输入内容',
      );
    }
  }

  Future<void> logout() async {
    try {
      await _auth.logout();
    } catch (_) {
      try {
        await _auth.clearSession();
      } catch (_) {
        // 即使本地清理异常，也不能让界面继续停留在已失效的登录态。
      }
    } finally {
      state = const SessionState(SessionPhase.unauthenticated);
    }
  }

  Future<bool> refreshAuthenticatedSession() async {
    try {
      final session = await _auth.restoreSession();
      if (session == null) {
        state = const SessionState(SessionPhase.unauthenticated);
        return false;
      }
      state = SessionState(SessionPhase.authenticated, session: session);
      return true;
    } catch (_) {
      state = const SessionState(SessionPhase.failure, message: '无法恢复安全会话');
      return false;
    }
  }

  void applyUser(UserIdentity user) {
    final session = state.session;
    if (session == null) return;
    state = SessionState(
      SessionPhase.authenticated,
      session: session.rebuild((builder) => builder.user.replace(user)),
    );
  }
}
