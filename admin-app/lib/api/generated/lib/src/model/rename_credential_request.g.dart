// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'rename_credential_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RenameCredentialRequest extends RenameCredentialRequest {
  @override
  final String name;
  @override
  final int version;

  factory _$RenameCredentialRequest(
          [void Function(RenameCredentialRequestBuilder)? updates]) =>
      (RenameCredentialRequestBuilder()..update(updates))._build();

  _$RenameCredentialRequest._({required this.name, required this.version})
      : super._();
  @override
  RenameCredentialRequest rebuild(
          void Function(RenameCredentialRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RenameCredentialRequestBuilder toBuilder() =>
      RenameCredentialRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RenameCredentialRequest &&
        name == other.name &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RenameCredentialRequest')
          ..add('name', name)
          ..add('version', version))
        .toString();
  }
}

class RenameCredentialRequestBuilder
    implements
        Builder<RenameCredentialRequest, RenameCredentialRequestBuilder> {
  _$RenameCredentialRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  RenameCredentialRequestBuilder() {
    RenameCredentialRequest._defaults(this);
  }

  RenameCredentialRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RenameCredentialRequest other) {
    _$v = other as _$RenameCredentialRequest;
  }

  @override
  void update(void Function(RenameCredentialRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RenameCredentialRequest build() => _build();

  _$RenameCredentialRequest _build() {
    final _$result = _$v ??
        _$RenameCredentialRequest._(
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'RenameCredentialRequest', 'name'),
          version: BuiltValueNullFieldError.checkNotNull(
              version, r'RenameCredentialRequest', 'version'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
