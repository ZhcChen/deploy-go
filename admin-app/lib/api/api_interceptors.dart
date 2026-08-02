import 'package:dio/dio.dart';

class RedactingLogInterceptor extends Interceptor {
  RedactingLogInterceptor(this._sink);

  final void Function(String message) _sink;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    _sink('${options.method} ${options.path}');
    handler.next(options);
  }

  @override
  void onResponse(
    Response<dynamic> response,
    ResponseInterceptorHandler handler,
  ) {
    _sink('${response.statusCode} ${response.requestOptions.path}');
    handler.next(response);
  }

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) {
    _sink(
      '${err.response?.statusCode ?? 'network'} ${err.requestOptions.path}',
    );
    handler.next(err);
  }
}

class UnauthorizedInterceptor extends Interceptor {
  UnauthorizedInterceptor(this._onUnauthorized);

  final void Function() _onUnauthorized;

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) {
    final path = err.requestOptions.path;
    if (err.response?.statusCode == 401 &&
        path != '/api/v1/auth/login' &&
        path != '/api/v1/setup') {
      _onUnauthorized();
    }
    handler.next(err);
  }
}
