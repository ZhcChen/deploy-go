import 'dart:async';
import 'dart:convert';

import 'package:deploy_go_api_client/deploy_go_api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/mobile_data_gateway.dart';
import '../../api/sse_client.dart';
import '../../api/contracts.dart';
import '../../app/providers.dart';
import '../shared/cursor_collection.dart';

final deploymentsProvider =
    StateNotifierProvider.autoDispose<
      CursorCollectionController<DeploymentResponse>,
      CursorCollectionState<DeploymentResponse>
    >((ref) {
      final controller = CursorCollectionController<DeploymentResponse>(
        (after) =>
            ref.read(mobileDataGatewayProvider).deployments(after: after),
        (item) => item.id,
        clearItemsOnRefreshError: true,
      );
      controller.refresh();
      return controller;
    });

final deploymentDetailProvider = StateNotifierProvider.autoDispose
    .family<DeploymentDetailController, DeploymentDetailState, String>((
      ref,
      id,
    ) {
      final controller = DeploymentDetailController(
        id,
        ref.read(mobileDataGatewayProvider),
        ref.read(deploymentSseClientProvider),
      );
      controller.initialize();
      return controller;
    });

class DeploymentDetailState {
  const DeploymentDetailState({
    this.deployment,
    this.logs = const <DeploymentLogResponse>[],
    this.lastEventId = 0,
    this.loading = true,
    this.connection = SseConnectionState.connecting,
    this.error,
    this.actionError,
    this.failedAction,
    this.action,
  });

  final DeploymentResponse? deployment;
  final List<DeploymentLogResponse> logs;
  final int lastEventId;
  final bool loading;
  final SseConnectionState connection;
  final Object? error;
  final Object? actionError;
  final String? failedAction;
  final String? action;

  DeploymentDetailState copyWith({
    DeploymentResponse? deployment,
    bool clearDeployment = false,
    List<DeploymentLogResponse>? logs,
    int? lastEventId,
    bool? loading,
    SseConnectionState? connection,
    Object? error,
    bool clearError = false,
    Object? actionError,
    bool clearActionError = false,
    String? failedAction,
    String? action,
    bool clearAction = false,
  }) => DeploymentDetailState(
    deployment: clearDeployment ? null : deployment ?? this.deployment,
    logs: logs ?? this.logs,
    lastEventId: lastEventId ?? this.lastEventId,
    loading: loading ?? this.loading,
    connection: connection ?? this.connection,
    error: clearError ? null : error ?? this.error,
    actionError: clearActionError ? null : actionError ?? this.actionError,
    failedAction: clearActionError ? null : failedAction ?? this.failedAction,
    action: clearAction ? null : action ?? this.action,
  );
}

class DeploymentDetailController extends StateNotifier<DeploymentDetailState> {
  DeploymentDetailController(this.id, this._gateway, this._sse)
    : super(const DeploymentDetailState());

  final String id;
  final MobileDataGateway _gateway;
  final DeploymentSseClient _sse;
  StreamSubscription<SseEvent>? _subscription;
  final Map<int, DeploymentLogResponse> _pendingLogs =
      <int, DeploymentLogResponse>{};
  Timer? _logFlushTimer;
  int _pendingLastEventId = 0;
  bool _foreground = true;
  int _generation = 0;

  Future<void> initialize() => refresh(reconnect: true);

  Future<void> refresh({bool reconnect = false}) async {
    final generation = ++_generation;
    state = state.copyWith(loading: state.deployment == null, clearError: true);
    try {
      final deployment = await _gateway.deployment(id);
      if (!mounted || generation != _generation) return;
      state = state.copyWith(deployment: deployment, loading: false);
      if (reconnect &&
          _foreground &&
          !isTerminalDeployment(deployment.status)) {
        await _connect();
      } else if (isTerminalDeployment(deployment.status)) {
        _flushLogs();
        state = state.copyWith(connection: SseConnectionState.ended);
        await _disconnect();
        if (!mounted || generation != _generation) return;
      }
    } catch (error) {
      if (!mounted || generation != _generation) return;
      final forbidden =
          error is ApiFailureException && error.failure.status == 403;
      if (forbidden) _clearPendingLogs();
      state = state.copyWith(
        clearDeployment: forbidden,
        logs: forbidden ? const <DeploymentLogResponse>[] : null,
        lastEventId: forbidden ? 0 : null,
        loading: false,
        error: error,
      );
      if (forbidden) await _disconnect();
    }
  }

  Future<void> enterBackground() async {
    _foreground = false;
    await _disconnect();
  }

  Future<void> enterForeground() async {
    _foreground = true;
    await refresh(reconnect: true);
  }

  Future<void> cancel() async {
    if (state.action != null) return;
    state = state.copyWith(action: 'cancel', clearActionError: true);
    try {
      final saved = await _gateway.cancelDeployment(id);
      if (!mounted) return;
      final terminal = isTerminalDeployment(saved.status);
      state = state.copyWith(
        deployment: saved,
        connection: terminal ? SseConnectionState.ended : null,
        clearAction: true,
      );
      if (terminal) await _disconnect();
    } catch (error) {
      if (mounted) {
        if (_isForbidden(error)) {
          await _revokeAccess(error);
        } else {
          state = state.copyWith(
            actionError: error,
            failedAction: 'cancel',
            clearAction: true,
          );
        }
      }
    }
  }

  Future<DeploymentResponse?> retry(String idempotencyKey) async {
    if (state.action != null) return null;
    state = state.copyWith(action: 'retry', clearActionError: true);
    try {
      final saved = await _gateway.retryDeployment(id, idempotencyKey);
      if (mounted) state = state.copyWith(clearAction: true);
      return saved;
    } catch (error) {
      if (mounted) {
        if (_isForbidden(error)) {
          await _revokeAccess(error);
        } else {
          state = state.copyWith(
            actionError: error,
            failedAction: 'retry',
            clearAction: true,
          );
        }
      }
      return null;
    }
  }

  Future<void> reconnect() => _connect();

  Future<void> _connect() async {
    await _disconnect();
    if (!mounted || !_foreground || state.deployment == null) return;
    state = state.copyWith(connection: SseConnectionState.connecting);
    _subscription = _sse
        .deploymentLogs(id, after: state.lastEventId)
        .listen(_handleEvent, onError: _handleStreamError);
  }

  void _handleEvent(SseEvent event) {
    if (!mounted) return;
    final eventId = int.tryParse(event.id) ?? state.lastEventId;
    if (event.event == 'stream-open') {
      state = state.copyWith(connection: SseConnectionState.open);
    } else if (event.event == 'stream-reconnecting') {
      state = state.copyWith(connection: SseConnectionState.reconnecting);
    } else if (event.event == 'log') {
      try {
        final json = jsonDecode(event.data) as Map<String, dynamic>;
        final sequence = json['sequence'] as int;
        if (_pendingLogs.containsKey(sequence) ||
            state.logs.any((item) => item.sequence == sequence)) {
          return;
        }
        final log = DeploymentLogResponse(
          (builder) => builder
            ..sequence = sequence
            ..stream = json['stream'] as String
            ..content = sanitizeLogText(json['content'] as String)
            ..truncated = json['truncated'] as bool
            ..createdAt = json['created_at'] as String,
        );
        _pendingLogs[sequence] = log;
        if (eventId > _pendingLastEventId) _pendingLastEventId = eventId;
        _logFlushTimer ??= Timer(const Duration(milliseconds: 16), _flushLogs);
      } catch (_) {
        state = state.copyWith(
          actionError: StateError('收到无法识别的日志事件'),
          failedAction: 'stream',
        );
      }
    } else if (event.event == 'terminal') {
      _flushLogs();
      state = state.copyWith(connection: SseConnectionState.ended);
      refresh();
    } else if (event.event == 'authorization-revoked') {
      var code = 'forbidden';
      var message = '日志访问授权已失效';
      var requestId = '';
      try {
        final data = jsonDecode(event.data) as Map<String, dynamic>;
        code = data['code'] as String? ?? code;
        message = data['message'] as String? ?? message;
        requestId = data['request_id'] as String? ?? requestId;
      } catch (_) {
        // 服务端旧事件可能没有结构化 data，仍需立即撤销本地访问。
      }
      _revokeAccess(
        ApiFailureException(
          ApiFailure(
            status: 403,
            code: code,
            message: message,
            requestId: requestId,
          ),
        ),
      );
    }
  }

  void _handleStreamError(Object error, StackTrace stackTrace) {
    if (!mounted || !_foreground) return;
    state = state.copyWith(
      connection: SseConnectionState.ended,
      actionError: error,
      failedAction: 'stream',
    );
  }

  bool _isForbidden(Object error) =>
      error is ApiFailureException && error.failure.status == 403;

  Future<void> _revokeAccess(Object error) async {
    if (!mounted) return;
    _clearPendingLogs();
    state = state.copyWith(
      clearDeployment: true,
      logs: const <DeploymentLogResponse>[],
      lastEventId: 0,
      connection: SseConnectionState.ended,
      error: error,
      clearAction: true,
      clearActionError: true,
    );
    await _disconnect();
  }

  Future<void> _disconnect() async {
    final subscription = _subscription;
    _subscription = null;
    await subscription?.cancel();
  }

  void _flushLogs() {
    _logFlushTimer?.cancel();
    _logFlushTimer = null;
    if (!mounted || _pendingLogs.isEmpty) return;
    final bySequence = <int, DeploymentLogResponse>{
      for (final log in state.logs) log.sequence: log,
      ..._pendingLogs,
    };
    final logs = bySequence.values.toList(growable: false)
      ..sort((left, right) => left.sequence.compareTo(right.sequence));
    final lastEventId = _pendingLastEventId > state.lastEventId
        ? _pendingLastEventId
        : state.lastEventId;
    _pendingLogs.clear();
    _pendingLastEventId = 0;
    state = state.copyWith(
      logs: logs.length > 1000 ? logs.sublist(logs.length - 1000) : logs,
      lastEventId: lastEventId,
      connection: SseConnectionState.open,
    );
  }

  void _clearPendingLogs() {
    _logFlushTimer?.cancel();
    _logFlushTimer = null;
    _pendingLogs.clear();
    _pendingLastEventId = 0;
  }

  @override
  void dispose() {
    _clearPendingLogs();
    _subscription?.cancel();
    super.dispose();
  }
}

bool isTerminalDeployment(String status) => const <String>{
  'succeeded',
  'failed',
  'canceled',
  'interrupted',
}.contains(status);

String sanitizeLogText(String value) => String.fromCharCodes(
  value.runes.map((code) {
    final unsafeControl =
        code <= 8 ||
        code == 11 ||
        code == 12 ||
        (code >= 14 && code <= 31) ||
        (code >= 127 && code <= 159);
    final unsafeDirection =
        (code >= 0x202a && code <= 0x202e) ||
        (code >= 0x2066 && code <= 0x2069);
    return unsafeControl || unsafeDirection ? 0xfffd : code;
  }),
);
