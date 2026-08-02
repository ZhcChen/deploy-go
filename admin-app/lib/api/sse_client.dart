import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';

import 'contracts.dart';
import 'deploy_go_api.dart';
import 'mobile_data_gateway.dart';

enum SseConnectionState { connecting, open, reconnecting, ended }

class SseEvent {
  const SseEvent({required this.id, required this.event, required this.data});

  final String id;
  final String event;
  final String data;
}

abstract interface class DeploymentSseClient {
  Stream<SseEvent> deploymentLogs(String deploymentId, {int after = 0});
}

class DioDeploymentSseClient implements DeploymentSseClient {
  DioDeploymentSseClient(this._api, {this.maxRetries = 5});

  final DeployGoApi _api;
  final int maxRetries;

  @override
  Stream<SseEvent> deploymentLogs(String deploymentId, {int after = 0}) {
    var cursor = after;
    var attempts = 0;
    var closed = false;
    CancelToken? request;
    StreamSubscription<SseEvent>? events;
    Timer? retryTimer;
    late final StreamController<SseEvent> controller;
    late Future<void> Function() connect;
    late void Function(Object error, StackTrace stackTrace) retryOrClose;

    retryOrClose = (error, stackTrace) {
      if (closed) return;
      if (attempts >= maxRetries) {
        closed = true;
        controller.addError(_normalizeSseError(error), stackTrace);
        controller.close();
        return;
      }
      final milliseconds = (1000 * (1 << attempts)).clamp(1000, 8000);
      attempts += 1;
      controller.add(
        SseEvent(id: '$cursor', event: 'stream-reconnecting', data: ''),
      );
      retryTimer = Timer(Duration(milliseconds: milliseconds), connect);
    };

    connect = () async {
      if (closed) return;
      retryTimer = null;
      request = CancelToken();
      try {
        final response = await _api.dio.get<ResponseBody>(
          '/api/v1/deployments/${Uri.encodeComponent(deploymentId)}/logs',
          options: Options(
            responseType: ResponseType.stream,
            headers: <String, Object>{
              'Accept': 'text/event-stream',
              if (cursor > 0) 'Last-Event-ID': '$cursor',
            },
          ),
          cancelToken: request,
        );
        if (closed) return;
        final body = response.data;
        if (body == null) throw StateError('日志响应没有可读取内容');
        attempts = 0;
        controller.add(SseEvent(id: '$cursor', event: 'stream-open', data: ''));
        events = parseSse(body.stream).listen(
          (event) {
            final next = int.tryParse(event.id);
            if (next != null && next >= 0 && next > cursor) cursor = next;
            controller.add(event);
            if (event.event == 'terminal' ||
                event.event == 'authorization-revoked') {
              closed = true;
              events?.cancel();
              controller.close();
            }
          },
          onError: retryOrClose,
          onDone: () {
            if (!closed) {
              retryOrClose(StateError('日志连接意外结束'), StackTrace.current);
            }
          },
          cancelOnError: true,
        );
      } catch (error, stackTrace) {
        final canceled = error is DioException && CancelToken.isCancel(error);
        final forbidden =
            error is DioException && error.response?.statusCode == 403;
        if (!closed && forbidden) {
          closed = true;
          final requestId = error.response?.headers.value('x-request-id') ?? '';
          controller.add(
            SseEvent(
              id: '$cursor',
              event: 'authorization-revoked',
              data: jsonEncode(<String, String>{
                'code': 'forbidden',
                'message': '日志访问授权已失效',
                'request_id': requestId,
              }),
            ),
          );
          await controller.close();
        } else if (!closed && !canceled) {
          retryOrClose(error, stackTrace);
        }
      }
    };

    controller = StreamController<SseEvent>(
      onListen: connect,
      onCancel: () async {
        closed = true;
        retryTimer?.cancel();
        request?.cancel('页面已离开或应用进入后台');
        await events?.cancel();
      },
    );
    return controller.stream;
  }
}

Object _normalizeSseError(Object error) {
  if (error is! DioException || error.response == null) return error;
  final response = error.response!;
  return ApiFailureException(
    ApiFailure(
      status: response.statusCode ?? 0,
      code: 'sse_http_error',
      message: '日志连接请求失败',
      requestId: response.headers.value('x-request-id') ?? '',
    ),
  );
}

Stream<SseEvent> parseSse(Stream<List<int>> source) async* {
  var buffer = '';
  var pendingCarriageReturn = false;
  await for (final chunk in source.transform(utf8.decoder)) {
    final normalized = StringBuffer();
    for (final code in chunk.codeUnits) {
      if (pendingCarriageReturn) {
        normalized.writeCharCode(0x0a);
        pendingCarriageReturn = false;
        if (code == 0x0a) continue;
      }
      if (code == 0x0d) {
        pendingCarriageReturn = true;
      } else {
        normalized.writeCharCode(code);
      }
    }
    buffer += normalized.toString();
    var boundary = buffer.indexOf('\n\n');
    while (boundary >= 0) {
      final block = buffer.substring(0, boundary);
      buffer = buffer.substring(boundary + 2);
      final event = _parseBlock(block);
      if (event != null) yield event;
      boundary = buffer.indexOf('\n\n');
    }
  }
  if (pendingCarriageReturn) buffer += '\n';
  final event = _parseBlock(buffer);
  if (event != null) yield event;
}

SseEvent? _parseBlock(String block) {
  if (block.trim().isEmpty) return null;
  var id = '';
  var event = 'message';
  final data = <String>[];
  for (final line in block.split('\n')) {
    if (line.startsWith(':')) continue;
    final separator = line.indexOf(':');
    final field = separator < 0 ? line : line.substring(0, separator);
    var value = separator < 0 ? '' : line.substring(separator + 1);
    if (value.startsWith(' ')) value = value.substring(1);
    switch (field) {
      case 'id':
        if (!value.contains('\u0000')) id = value;
      case 'event':
        event = value;
      case 'data':
        data.add(value);
    }
  }
  if (data.isEmpty) return null;
  return SseEvent(id: id, event: event, data: data.join('\n'));
}
