// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_status_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateStatusRequest extends UpdateStatusRequest {
  @override
  final String status;
  @override
  final int version;

  factory _$UpdateStatusRequest([
    void Function(UpdateStatusRequestBuilder)? updates,
  ]) => (UpdateStatusRequestBuilder()..update(updates))._build();

  _$UpdateStatusRequest._({required this.status, required this.version})
    : super._();
  @override
  UpdateStatusRequest rebuild(
    void Function(UpdateStatusRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdateStatusRequestBuilder toBuilder() =>
      UpdateStatusRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateStatusRequest &&
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
    return (newBuiltValueToStringHelper(r'UpdateStatusRequest')
          ..add('status', status)
          ..add('version', version))
        .toString();
  }
}

class UpdateStatusRequestBuilder
    implements Builder<UpdateStatusRequest, UpdateStatusRequestBuilder> {
  _$UpdateStatusRequest? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  UpdateStatusRequestBuilder() {
    UpdateStatusRequest._defaults(this);
  }

  UpdateStatusRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateStatusRequest other) {
    _$v = other as _$UpdateStatusRequest;
  }

  @override
  void update(void Function(UpdateStatusRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateStatusRequest build() => _build();

  _$UpdateStatusRequest _build() {
    final _$result =
        _$v ??
        _$UpdateStatusRequest._(
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'UpdateStatusRequest',
            'status',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'UpdateStatusRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
