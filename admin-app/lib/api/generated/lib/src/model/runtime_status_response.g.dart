// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_status_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeStatusResponse extends RuntimeStatusResponse {
  @override
  final String applicationId;
  @override
  final String createdAt;
  @override
  final String? errorCode;
  @override
  final String? errorMessage;
  @override
  final String? observedAt;
  @override
  final JsonObject? payload;
  @override
  final String requestedAt;
  @override
  final String? requestedBy;
  @override
  final String runtimeStatusId;
  @override
  final String status;
  @override
  final String targetCode;
  @override
  final String targetId;
  @override
  final String updatedAt;

  factory _$RuntimeStatusResponse([
    void Function(RuntimeStatusResponseBuilder)? updates,
  ]) => (RuntimeStatusResponseBuilder()..update(updates))._build();

  _$RuntimeStatusResponse._({
    required this.applicationId,
    required this.createdAt,
    this.errorCode,
    this.errorMessage,
    this.observedAt,
    this.payload,
    required this.requestedAt,
    this.requestedBy,
    required this.runtimeStatusId,
    required this.status,
    required this.targetCode,
    required this.targetId,
    required this.updatedAt,
  }) : super._();
  @override
  RuntimeStatusResponse rebuild(
    void Function(RuntimeStatusResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeStatusResponseBuilder toBuilder() =>
      RuntimeStatusResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeStatusResponse &&
        applicationId == other.applicationId &&
        createdAt == other.createdAt &&
        errorCode == other.errorCode &&
        errorMessage == other.errorMessage &&
        observedAt == other.observedAt &&
        payload == other.payload &&
        requestedAt == other.requestedAt &&
        requestedBy == other.requestedBy &&
        runtimeStatusId == other.runtimeStatusId &&
        status == other.status &&
        targetCode == other.targetCode &&
        targetId == other.targetId &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, errorMessage.hashCode);
    _$hash = $jc(_$hash, observedAt.hashCode);
    _$hash = $jc(_$hash, payload.hashCode);
    _$hash = $jc(_$hash, requestedAt.hashCode);
    _$hash = $jc(_$hash, requestedBy.hashCode);
    _$hash = $jc(_$hash, runtimeStatusId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, targetCode.hashCode);
    _$hash = $jc(_$hash, targetId.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeStatusResponse')
          ..add('applicationId', applicationId)
          ..add('createdAt', createdAt)
          ..add('errorCode', errorCode)
          ..add('errorMessage', errorMessage)
          ..add('observedAt', observedAt)
          ..add('payload', payload)
          ..add('requestedAt', requestedAt)
          ..add('requestedBy', requestedBy)
          ..add('runtimeStatusId', runtimeStatusId)
          ..add('status', status)
          ..add('targetCode', targetCode)
          ..add('targetId', targetId)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class RuntimeStatusResponseBuilder
    implements Builder<RuntimeStatusResponse, RuntimeStatusResponseBuilder> {
  _$RuntimeStatusResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  String? _errorMessage;
  String? get errorMessage => _$this._errorMessage;
  set errorMessage(String? errorMessage) => _$this._errorMessage = errorMessage;

  String? _observedAt;
  String? get observedAt => _$this._observedAt;
  set observedAt(String? observedAt) => _$this._observedAt = observedAt;

  JsonObject? _payload;
  JsonObject? get payload => _$this._payload;
  set payload(JsonObject? payload) => _$this._payload = payload;

  String? _requestedAt;
  String? get requestedAt => _$this._requestedAt;
  set requestedAt(String? requestedAt) => _$this._requestedAt = requestedAt;

  String? _requestedBy;
  String? get requestedBy => _$this._requestedBy;
  set requestedBy(String? requestedBy) => _$this._requestedBy = requestedBy;

  String? _runtimeStatusId;
  String? get runtimeStatusId => _$this._runtimeStatusId;
  set runtimeStatusId(String? runtimeStatusId) =>
      _$this._runtimeStatusId = runtimeStatusId;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _targetCode;
  String? get targetCode => _$this._targetCode;
  set targetCode(String? targetCode) => _$this._targetCode = targetCode;

  String? _targetId;
  String? get targetId => _$this._targetId;
  set targetId(String? targetId) => _$this._targetId = targetId;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  RuntimeStatusResponseBuilder() {
    RuntimeStatusResponse._defaults(this);
  }

  RuntimeStatusResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _createdAt = $v.createdAt;
      _errorCode = $v.errorCode;
      _errorMessage = $v.errorMessage;
      _observedAt = $v.observedAt;
      _payload = $v.payload;
      _requestedAt = $v.requestedAt;
      _requestedBy = $v.requestedBy;
      _runtimeStatusId = $v.runtimeStatusId;
      _status = $v.status;
      _targetCode = $v.targetCode;
      _targetId = $v.targetId;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeStatusResponse other) {
    _$v = other as _$RuntimeStatusResponse;
  }

  @override
  void update(void Function(RuntimeStatusResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeStatusResponse build() => _build();

  _$RuntimeStatusResponse _build() {
    final _$result =
        _$v ??
        _$RuntimeStatusResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
            applicationId,
            r'RuntimeStatusResponse',
            'applicationId',
          ),
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'RuntimeStatusResponse',
            'createdAt',
          ),
          errorCode: errorCode,
          errorMessage: errorMessage,
          observedAt: observedAt,
          payload: payload,
          requestedAt: BuiltValueNullFieldError.checkNotNull(
            requestedAt,
            r'RuntimeStatusResponse',
            'requestedAt',
          ),
          requestedBy: requestedBy,
          runtimeStatusId: BuiltValueNullFieldError.checkNotNull(
            runtimeStatusId,
            r'RuntimeStatusResponse',
            'runtimeStatusId',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'RuntimeStatusResponse',
            'status',
          ),
          targetCode: BuiltValueNullFieldError.checkNotNull(
            targetCode,
            r'RuntimeStatusResponse',
            'targetCode',
          ),
          targetId: BuiltValueNullFieldError.checkNotNull(
            targetId,
            r'RuntimeStatusResponse',
            'targetId',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'RuntimeStatusResponse',
            'updatedAt',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
