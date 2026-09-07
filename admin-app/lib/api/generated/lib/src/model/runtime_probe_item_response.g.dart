// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_probe_item_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeProbeItemResponse extends RuntimeProbeItemResponse {
  @override
  final String applicationId;
  @override
  final String? errorCode;
  @override
  final String? errorMessage;
  @override
  final String? runtimeStatusId;
  @override
  final String status;

  factory _$RuntimeProbeItemResponse([
    void Function(RuntimeProbeItemResponseBuilder)? updates,
  ]) => (RuntimeProbeItemResponseBuilder()..update(updates))._build();

  _$RuntimeProbeItemResponse._({
    required this.applicationId,
    this.errorCode,
    this.errorMessage,
    this.runtimeStatusId,
    required this.status,
  }) : super._();
  @override
  RuntimeProbeItemResponse rebuild(
    void Function(RuntimeProbeItemResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeProbeItemResponseBuilder toBuilder() =>
      RuntimeProbeItemResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeProbeItemResponse &&
        applicationId == other.applicationId &&
        errorCode == other.errorCode &&
        errorMessage == other.errorMessage &&
        runtimeStatusId == other.runtimeStatusId &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, errorMessage.hashCode);
    _$hash = $jc(_$hash, runtimeStatusId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeProbeItemResponse')
          ..add('applicationId', applicationId)
          ..add('errorCode', errorCode)
          ..add('errorMessage', errorMessage)
          ..add('runtimeStatusId', runtimeStatusId)
          ..add('status', status))
        .toString();
  }
}

class RuntimeProbeItemResponseBuilder
    implements
        Builder<RuntimeProbeItemResponse, RuntimeProbeItemResponseBuilder> {
  _$RuntimeProbeItemResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  String? _errorMessage;
  String? get errorMessage => _$this._errorMessage;
  set errorMessage(String? errorMessage) => _$this._errorMessage = errorMessage;

  String? _runtimeStatusId;
  String? get runtimeStatusId => _$this._runtimeStatusId;
  set runtimeStatusId(String? runtimeStatusId) =>
      _$this._runtimeStatusId = runtimeStatusId;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  RuntimeProbeItemResponseBuilder() {
    RuntimeProbeItemResponse._defaults(this);
  }

  RuntimeProbeItemResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _errorCode = $v.errorCode;
      _errorMessage = $v.errorMessage;
      _runtimeStatusId = $v.runtimeStatusId;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeProbeItemResponse other) {
    _$v = other as _$RuntimeProbeItemResponse;
  }

  @override
  void update(void Function(RuntimeProbeItemResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeProbeItemResponse build() => _build();

  _$RuntimeProbeItemResponse _build() {
    final _$result =
        _$v ??
        _$RuntimeProbeItemResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
            applicationId,
            r'RuntimeProbeItemResponse',
            'applicationId',
          ),
          errorCode: errorCode,
          errorMessage: errorMessage,
          runtimeStatusId: runtimeStatusId,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'RuntimeProbeItemResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
