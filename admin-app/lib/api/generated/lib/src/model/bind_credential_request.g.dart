// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'bind_credential_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$BindCredentialRequest extends BindCredentialRequest {
  @override
  final String credentialId;
  @override
  final int version;

  factory _$BindCredentialRequest([
    void Function(BindCredentialRequestBuilder)? updates,
  ]) => (BindCredentialRequestBuilder()..update(updates))._build();

  _$BindCredentialRequest._({required this.credentialId, required this.version})
    : super._();
  @override
  BindCredentialRequest rebuild(
    void Function(BindCredentialRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  BindCredentialRequestBuilder toBuilder() =>
      BindCredentialRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is BindCredentialRequest &&
        credentialId == other.credentialId &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, credentialId.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'BindCredentialRequest')
          ..add('credentialId', credentialId)
          ..add('version', version))
        .toString();
  }
}

class BindCredentialRequestBuilder
    implements Builder<BindCredentialRequest, BindCredentialRequestBuilder> {
  _$BindCredentialRequest? _$v;

  String? _credentialId;
  String? get credentialId => _$this._credentialId;
  set credentialId(String? credentialId) => _$this._credentialId = credentialId;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  BindCredentialRequestBuilder() {
    BindCredentialRequest._defaults(this);
  }

  BindCredentialRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _credentialId = $v.credentialId;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(BindCredentialRequest other) {
    _$v = other as _$BindCredentialRequest;
  }

  @override
  void update(void Function(BindCredentialRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  BindCredentialRequest build() => _build();

  _$BindCredentialRequest _build() {
    final _$result =
        _$v ??
        _$BindCredentialRequest._(
          credentialId: BuiltValueNullFieldError.checkNotNull(
            credentialId,
            r'BindCredentialRequest',
            'credentialId',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'BindCredentialRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
