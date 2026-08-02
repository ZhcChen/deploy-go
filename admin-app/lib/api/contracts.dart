class ApiRequestContext {
  const ApiRequestContext({this.csrfToken});

  final String? csrfToken;
}

class CursorPage<T> {
  const CursorPage({required this.items, this.nextCursor});

  final List<T> items;
  final String? nextCursor;
}

class ApiFailure {
  const ApiFailure({
    required this.status,
    required this.code,
    required this.message,
    required this.requestId,
    this.details,
  });

  final int status;
  final String code;
  final String message;
  final String requestId;
  final Object? details;
}

class SseResumeCursor {
  const SseResumeCursor({this.lastEventId, this.after});

  final String? lastEventId;
  final int? after;
}

class SseMessage {
  const SseMessage({required this.id, required this.event, required this.data});

  final String id;
  final String event;
  final String data;
}
