//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

import 'dart:async';

import 'package:built_value/json_object.dart';
import 'package:built_value/serializer.dart';
import 'package:dio/dio.dart';

import 'package:deploy_go_api_client/src/api_util.dart';
import 'package:deploy_go_api_client/src/model/error_response.dart';

class RuntimeLogsApi {

  final Dio _dio;

  final Serializers _serializers;

  const RuntimeLogsApi(this._dio, this._serializers);

  /// runtimeLogsStream
  ///
  ///
  /// Parameters:
  /// * [after]
  /// * [level]
  /// * [requestId]
  /// * [target]
  /// * [cancelToken] - A [CancelToken] that can be used to cancel the operation
  /// * [headers] - Can be used to add additional headers to the request
  /// * [extras] - Can be used to add flags to the request
  /// * [validateStatus] - A [ValidateStatus] callback that can be used to determine request success based on the HTTP status of the response
  /// * [onSendProgress] - A [ProgressCallback] that can be used to get the send progress
  /// * [onReceiveProgress] - A [ProgressCallback] that can be used to get the receive progress
  ///
  /// Returns a [Future]
  /// Throws [DioException] if API call or serialization fails
  Future<Response<void>> runtimeLogsStream({
    int? after,
    String? level,
    String? requestId,
    String? target,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final _path = r'/api/v1/runtime-logs';
    final _options = Options(
      method: r'GET',
      headers: <String, dynamic>{
        ...?headers,
      },
      extra: <String, dynamic>{
        'secure': <Map<String, String>>[
          {
            'type': 'apiKey',
            'name': 'cookieAuth',
            'keyName': 'deploy_go_session',
            'where': '',
          },
        ],
        ...?extra,
      },
      validateStatus: validateStatus,
    );

    final _queryParameters = <String, dynamic>{
      if (after != null) r'after': encodeQueryParameter(_serializers, after, const FullType(int)),
      if (level != null) r'level': encodeQueryParameter(_serializers, level, const FullType(String)),
      if (requestId != null) r'request_id': encodeQueryParameter(_serializers, requestId, const FullType(String)),
      if (target != null) r'target': encodeQueryParameter(_serializers, target, const FullType(String)),
    };

    final _response = await _dio.request<Object>(
      _path,
      options: _options,
      queryParameters: _queryParameters,
      cancelToken: cancelToken,
      onSendProgress: onSendProgress,
      onReceiveProgress: onReceiveProgress,
    );

    return _response;
  }

}
