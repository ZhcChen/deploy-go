// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'target_status_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TargetStatusRequest extends TargetStatusRequest {
  @override
  final String status;
  @override
  final int version;

  factory _$TargetStatusRequest(
          [void Function(TargetStatusRequestBuilder)? updates]) =>
      (TargetStatusRequestBuilder()..update(updates))._build();

  _$TargetStatusRequest._({required this.status, required this.version})
      : super._();
  @override
  TargetStatusRequest rebuild(
          void Function(TargetStatusRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TargetStatusRequestBuilder toBuilder() =>
      TargetStatusRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TargetStatusRequest &&
        status == other.status &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TargetStatusRequest')
          ..add('status', status)
          ..add('version', version))
        .toString();
  }
}

class TargetStatusRequestBuilder
    implements Builder<TargetStatusRequest, TargetStatusRequestBuilder> {
  _$TargetStatusRequest? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  TargetStatusRequestBuilder() {
    TargetStatusRequest._defaults(this);
  }

  TargetStatusRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TargetStatusRequest other) {
    _$v = other as _$TargetStatusRequest;
  }

  @override
  void update(void Function(TargetStatusRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TargetStatusRequest build() => _build();

  _$TargetStatusRequest _build() {
    final _$result = _$v ??
        _$TargetStatusRequest._(
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'TargetStatusRequest', 'status'),
          version: BuiltValueNullFieldError.checkNotNull(
              version, r'TargetStatusRequest', 'version'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
