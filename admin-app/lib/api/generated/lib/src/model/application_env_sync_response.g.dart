// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_env_sync_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationEnvSyncResponse extends ApplicationEnvSyncResponse {
  @override
  final int? actualVersion;
  @override
  final String? errorCode;
  @override
  final String? errorMessage;
  @override
  final String? lastAttemptAt;
  @override
  final String nodeId;
  @override
  final String nodeName;
  @override
  final String status;
  @override
  final String? syncedAt;
  @override
  final String targetId;

  factory _$ApplicationEnvSyncResponse([
    void Function(ApplicationEnvSyncResponseBuilder)? updates,
  ]) => (ApplicationEnvSyncResponseBuilder()..update(updates))._build();

  _$ApplicationEnvSyncResponse._({
    this.actualVersion,
    this.errorCode,
    this.errorMessage,
    this.lastAttemptAt,
    required this.nodeId,
    required this.nodeName,
    required this.status,
    this.syncedAt,
    required this.targetId,
  }) : super._();
  @override
  ApplicationEnvSyncResponse rebuild(
    void Function(ApplicationEnvSyncResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationEnvSyncResponseBuilder toBuilder() =>
      ApplicationEnvSyncResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationEnvSyncResponse &&
        actualVersion == other.actualVersion &&
        errorCode == other.errorCode &&
        errorMessage == other.errorMessage &&
        lastAttemptAt == other.lastAttemptAt &&
        nodeId == other.nodeId &&
        nodeName == other.nodeName &&
        status == other.status &&
        syncedAt == other.syncedAt &&
        targetId == other.targetId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, actualVersion.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, errorMessage.hashCode);
    _$hash = $jc(_$hash, lastAttemptAt.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, nodeName.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, syncedAt.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationEnvSyncResponse')
          ..add('actualVersion', actualVersion)
          ..add('errorCode', errorCode)
          ..add('errorMessage', errorMessage)
          ..add('lastAttemptAt', lastAttemptAt)
          ..add('nodeId', nodeId)
          ..add('nodeName', nodeName)
          ..add('status', status)
          ..add('syncedAt', syncedAt)
          ..add('targetId', targetId))
        .toString();
  }
}

class ApplicationEnvSyncResponseBuilder
    implements
        Builder<ApplicationEnvSyncResponse, ApplicationEnvSyncResponseBuilder> {
  _$ApplicationEnvSyncResponse? _$v;

  int? _actualVersion;
  int? get actualVersion => _$this._actualVersion;
  set actualVersion(int? actualVersion) =>
      _$this._actualVersion = actualVersion;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  String? _errorMessage;
  String? get errorMessage => _$this._errorMessage;
  set errorMessage(String? errorMessage) => _$this._errorMessage = errorMessage;

  String? _lastAttemptAt;
  String? get lastAttemptAt => _$this._lastAttemptAt;
  set lastAttemptAt(String? lastAttemptAt) =>
      _$this._lastAttemptAt = lastAttemptAt;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _nodeName;
  String? get nodeName => _$this._nodeName;
  set nodeName(String? nodeName) => _$this._nodeName = nodeName;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _syncedAt;
  String? get syncedAt => _$this._syncedAt;
  set syncedAt(String? syncedAt) => _$this._syncedAt = syncedAt;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  ApplicationEnvSyncResponseBuilder() {
    ApplicationEnvSyncResponse._defaults(this);
  }

  ApplicationEnvSyncResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _actualVersion = $v.actualVersion;
      _errorCode = $v.errorCode;
      _errorMessage = $v.errorMessage;
      _lastAttemptAt = $v.lastAttemptAt;
      _nodeId = $v.nodeId;
      _nodeName = $v.nodeName;
      _status = $v.status;
      _syncedAt = $v.syncedAt;
      _targetId = $v.targetId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationEnvSyncResponse other) {
    _$v = other as _$ApplicationEnvSyncResponse;
  }

  @override
  void update(void Function(ApplicationEnvSyncResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationEnvSyncResponse build() => _build();

  _$ApplicationEnvSyncResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationEnvSyncResponse._(
          actualVersion: actualVersion,
          errorCode: errorCode,
          errorMessage: errorMessage,
          lastAttemptAt: lastAttemptAt,
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'ApplicationEnvSyncResponse',
            'nodeId',
          ),
          nodeName: BuiltValueNullFieldError.checkNotNull(
            nodeName,
            r'ApplicationEnvSyncResponse',
            'nodeName',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ApplicationEnvSyncResponse',
            'status',
          ),
          syncedAt: syncedAt,
          targetId: BuiltValueNullFieldError.checkNotNull(
            targetId,
            r'ApplicationEnvSyncResponse',
            'targetId',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
