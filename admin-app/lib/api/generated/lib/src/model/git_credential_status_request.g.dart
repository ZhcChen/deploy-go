// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'git_credential_status_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GitCredentialStatusRequest extends GitCredentialStatusRequest {
  @override
  final String status;
  @override
  final int version;

  factory _$GitCredentialStatusRequest([
    void Function(GitCredentialStatusRequestBuilder)? updates,
  ]) => (GitCredentialStatusRequestBuilder()..update(updates))._build();

  _$GitCredentialStatusRequest._({required this.status, required this.version})
    : super._();
  @override
  GitCredentialStatusRequest rebuild(
    void Function(GitCredentialStatusRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  GitCredentialStatusRequestBuilder toBuilder() =>
      GitCredentialStatusRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GitCredentialStatusRequest &&
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
    return (newBuiltValueToStringHelper(r'GitCredentialStatusRequest')
          ..add('status', status)
          ..add('version', version))
        .toString();
  }
}

class GitCredentialStatusRequestBuilder
    implements
        Builder<GitCredentialStatusRequest, GitCredentialStatusRequestBuilder> {
  _$GitCredentialStatusRequest? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  GitCredentialStatusRequestBuilder() {
    GitCredentialStatusRequest._defaults(this);
  }

  GitCredentialStatusRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GitCredentialStatusRequest other) {
    _$v = other as _$GitCredentialStatusRequest;
  }

  @override
  void update(void Function(GitCredentialStatusRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GitCredentialStatusRequest build() => _build();

  _$GitCredentialStatusRequest _build() {
    final _$result =
        _$v ??
        _$GitCredentialStatusRequest._(
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'GitCredentialStatusRequest',
            'status',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'GitCredentialStatusRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
