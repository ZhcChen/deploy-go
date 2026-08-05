import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:dio/dio.dart';

import '../security/secure_session_store.dart';
import 'deploy_go_api.dart';

abstract interface class AuthGateway {
  Stream<void> get unauthorized;
  Future<SetupStatusResponse> setupStatus();
  Future<SessionResponse> login({
    required String username,
    required String password,
  });
  Future<UserIdentity> setup({
    required String username,
    required String password,
    required String displayName,
  });
  Future<SessionResponse?> restoreSession();
  Future<void> logout();
  Future<void> clearSession();
}

class AuthRepository implements AuthGateway {
  AuthRepository(this._api, this._sessionStore);

  final DeployGoApi _api;
  final SecureSessionStore _sessionStore;

  @override
  Stream<void> get unauthorized => _api.unauthorized;

  @override
  Future<SetupStatusResponse> setupStatus() async {
    final response = await _api.client.getAuthApi().authSetupStatus();
    return _required(response.data);
  }

  @override
  Future<SessionResponse> login({
    required String username,
    required String password,
  }) async {
    final response = await _api.client.getAuthApi().authLogin(
      origin: _api.environment.allowedOrigin,
      loginRequest: LoginRequest(
        (builder) => builder
          ..username = username
          ..password = password,
      ),
    );
    final session = _required(response.data);
    await _sessionStore.writeCsrfToken(session.csrfToken);
    return session;
  }

  @override
  Future<UserIdentity> setup({
    required String username,
    required String password,
    required String displayName,
  }) async {
    final response = await _api.client.getAuthApi().authSetup(
      origin: _api.environment.allowedOrigin,
      setupRequest: SetupRequest(
        (builder) => builder
          ..username = username
          ..password = password
          ..displayName = displayName,
      ),
    );
    return _required(response.data);
  }

  @override
  Future<SessionResponse?> restoreSession() async {
    try {
      final user = _required((await _api.client.getAuthApi().authMe()).data);
      final csrf = _required(
        (await _api.client.getAuthApi().authRefreshCsrf(
          origin: _api.environment.allowedOrigin,
          secFetchSite: 'same-origin',
          secFetchMode: 'cors',
        )).data,
      );
      await _sessionStore.writeCsrfToken(csrf.csrfToken);
      return SessionResponse(
        (builder) => builder
          ..csrfToken = csrf.csrfToken
          ..user.replace(user),
      );
    } on DioException catch (error) {
      if (error.response?.statusCode == 401) {
        await clearSession();
        return null;
      }
      rethrow;
    }
  }

  @override
  Future<void> logout() async {
    final csrf = await _sessionStore.readCsrfToken();
    try {
      if (csrf != null) {
        await _api.client.getAuthApi().authLogout(xCSRFToken: csrf);
      }
    } finally {
      await clearSession();
    }
  }

  @override
  Future<void> clearSession() async {
    await _api.clearCookies();
    await _sessionStore.clearCsrfToken();
  }
}

T _required<T>(T? value) {
  if (value == null) throw StateError('API 返回缺少必要数据');
  return value;
}
