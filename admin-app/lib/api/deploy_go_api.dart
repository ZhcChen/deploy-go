import 'dart:async';

import 'package:cookie_jar/cookie_jar.dart';
import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:dio/dio.dart';
import 'package:dio_cookie_manager/dio_cookie_manager.dart';

import '../security/secure_cookie_storage.dart';
import '../security/secure_key_value_store.dart';
import 'api_environment.dart';
import 'api_interceptors.dart';

class DeployGoApi {
  DeployGoApi._({
    required this.environment,
    required this.client,
    required this.cookieJar,
    required StreamController<void> unauthorizedController,
  }) : _unauthorizedController = unauthorizedController;

  final ApiEnvironment environment;
  final DeployGoApiClient client;
  final PersistCookieJar cookieJar;
  final StreamController<void> _unauthorizedController;
  Stream<void> get unauthorized => _unauthorizedController.stream;

  static Future<DeployGoApi> create({
    required ApiEnvironment environment,
    required SecureKeyValueStore secureStore,
    HttpClientAdapter? adapter,
    void Function(String message)? logSink,
  }) async {
    final checkedEnvironment = environment.validated();
    final cookieJar = PersistCookieJar(
      persistSession: true,
      storage: SecureCookieStorage(secureStore),
    );
    await cookieJar.forceInit();
    final dio = Dio(
      BaseOptions(
        baseUrl: checkedEnvironment.baseUrl,
        connectTimeout: const Duration(seconds: 5),
        receiveTimeout: const Duration(seconds: 15),
        headers: const <String, Object>{'Accept': 'application/json'},
      ),
    );
    if (adapter != null) dio.httpClientAdapter = adapter;
    dio.interceptors.add(CookieManager(cookieJar));
    final unauthorizedController = StreamController<void>.broadcast();
    dio.interceptors.add(
      UnauthorizedInterceptor(() => unauthorizedController.add(null)),
    );
    dio.interceptors.add(RedactingLogInterceptor(logSink ?? (_) {}));
    final client = DeployGoApiClient(
      dio: dio,
      basePathOverride: checkedEnvironment.baseUrl,
      interceptors: const <Interceptor>[],
    );
    return DeployGoApi._(
      environment: checkedEnvironment,
      client: client,
      cookieJar: cookieJar,
      unauthorizedController: unauthorizedController,
    );
  }

  Future<void> clearCookies() => cookieJar.deleteAll();

  Future<void> dispose() => _unauthorizedController.close();
}
