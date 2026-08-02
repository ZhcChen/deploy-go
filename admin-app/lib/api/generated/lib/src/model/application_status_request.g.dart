// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_status_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationStatusRequest extends ApplicationStatusRequest {
  @override
  final String status;
  @override
  final int version;

  factory _$ApplicationStatusRequest([
    void Function(ApplicationStatusRequestBuilder)? updates,
  ]) => (ApplicationStatusRequestBuilder()..update(updates))._build();

  _$ApplicationStatusRequest._({required this.status, required this.version})
    : super._();
  @override
  ApplicationStatusRequest rebuild(
    void Function(ApplicationStatusRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationStatusRequestBuilder toBuilder() =>
      ApplicationStatusRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationStatusRequest &&
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
    return (newBuiltValueToStringHelper(r'ApplicationStatusRequest')
          ..add('status', status)
          ..add('version', version))
        .toString();
  }
}

class ApplicationStatusRequestBuilder
    implements
        Builder<ApplicationStatusRequest, ApplicationStatusRequestBuilder> {
  _$ApplicationStatusRequest? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ApplicationStatusRequestBuilder() {
    ApplicationStatusRequest._defaults(this);
  }

  ApplicationStatusRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationStatusRequest other) {
    _$v = other as _$ApplicationStatusRequest;
  }

  @override
  void update(void Function(ApplicationStatusRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationStatusRequest build() => _build();

  _$ApplicationStatusRequest _build() {
    final _$result =
        _$v ??
        _$ApplicationStatusRequest._(
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ApplicationStatusRequest',
            'status',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'ApplicationStatusRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
